use crate::omdb::{MediaType, SearchResult};
use crate::{ArgsError, YearParseError};
use clap::ArgMatches;
use lazy_static::lazy_static;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::min;
use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;

lazy_static! {
    // I'm so sorry, this is my compromise for easily getting the current year
    static ref CURRENT_YEAR: u16 = {
        use std::time::SystemTime;
        let timestamp = humantime::format_rfc3339(SystemTime::now())
            .to_string();
        timestamp.split_once('-').unwrap().0.parse().expect("Bad current year")
    };
}

#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Filters {
    pub types: MediaType,
    pub years: Option<Year>,
}

impl Filters {
    pub fn new(clap_matches: &ArgMatches) -> Result<Self, ArgsError> {
        let types = match clap_matches.values_of("filter_type") {
            Some(vals) => {
                let mut mt = MediaType::empty();
                for s in vals {
                    mt |= MediaType::from_str(s)?;
                }
                mt
            }
            None => MediaType::ALL,
        };

        // Match used so ? can be used
        let years = match clap_matches.value_of("filter_year") {
            Some(year_str) => Some(Year::from_str(year_str)?),
            None => None,
        };

        Ok(Filters { types, years })
    }

    pub fn allows(&self, search_result: &SearchResult) -> bool {
        let year_matches = self
            .years
            .as_ref()
            .map(|year| year.contains(&search_result.year))
            .unwrap_or(true);
        let media_type_matches = self.types.contains(search_result.media_type);
        year_matches && media_type_matches
    }

    pub fn combinations(&self) -> usize {
        let types = if self.types.is_all() {
            1
        } else {
            self.types.count()
        };
        let years = self.years.as_ref().map(|year| year.0.len()).unwrap_or(1);
        types * years
    }
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            types: MediaType::ALL,
            years: None,
        }
    }
}

// Limitation: series' are assumed to end in the current year
// Fortunately due to the Display impl the user won't see this
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Year(pub(crate) RangeInclusive<u16>);

impl Year {
    const SEPARATORS: [char; 2] = ['-', '–'];

    pub fn contains(&self, year: &Year) -> bool {
        self.0.start() <= year.0.end() && year.0.start() <= self.0.end()
    }

    fn is_single(&self) -> bool {
        self.0.start() == self.0.end()
    }
}

impl FromStr for Year {
    type Err = YearParseError;

    // WARNING: not all separators are one byte, this must not be assumed!
    fn from_str(year_str: &str) -> Result<Self, Self::Err> {
        use YearParseError::*;

        match year_str.split_once(&Year::SEPARATORS[..]) {
            Some((start_str, end_str)) => {
                let mut start = if !start_str.is_empty() {
                    let start = u16::from_str(start_str)?;
                    // Make sure start isn't in the future
                    if start > *CURRENT_YEAR {
                        return Err(StartInFuture);
                    }
                    start
                } else {
                    0
                };

                let mut end = if !end_str.is_empty() {
                    let mut end = u16::from_str(end_str)?;
                    // Make sure arg isn't bigger than current year
                    if end > *CURRENT_YEAR {
                        eprintln!("WARNING: using current year for end of date range instead");
                        end = *CURRENT_YEAR;
                    }
                    end
                } else if start_str.is_empty() {
                    return Err(NoYearsSpecified);
                } else {
                    *CURRENT_YEAR
                };

                // Save the user from their silliness
                if end < start {
                    eprintln!("WARNING: looks like you put the date range in backwards, fixed that for you");
                    std::mem::swap(&mut start, &mut end);
                }

                Ok(Year(start..=end))
            }
            None => {
                // Should be just a year we can parse
                let year = min(u16::from_str(year_str)?, *CURRENT_YEAR);
                Ok(Year(year..=year))
            }
        }
    }
}

impl Serialize for Year {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Year {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        Year::from_str(&s).map_err(|e| {
            D::Error::custom(format!("could not parse field as year: {e:?}"))
        })
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_single() {
            write!(f, "{}", self.0.start())
        } else {
            let start = *self.0.start();
            let end = *self.0.end();
            if start != 0 {
                write!(f, "{start}")?;
            }
            write!(f, "-")?;
            if end != *CURRENT_YEAR {
                write!(f, "{end}")?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod filters_unit_tests {
    use crate::omdb::MediaType;
    use crate::{Filters, Year};

    #[test]
    fn combinations() {
        let filters = vec![
            Filters::default(),
            Filters {
                years: Some(Year(1960..=1970)),
                ..Default::default()
            },
            Filters {
                types: MediaType::SERIES,
                years: Some(Year(1985..=2000)),
            },
            Filters {
                types: MediaType::MOVIE,
                years: Some(Year(1980..=2000)),
            },
        ];
        let expected: Vec<usize> = vec![1, 11, 16, 21];

        filters
            .iter()
            .zip(expected)
            .for_each(|(filters, expected)| {
                assert_eq!(
                    filters.combinations(),
                    expected,
                    "Expected {expected} combination(s) from {:#?}",
                    filters,
                );
            });
    }

    mod creation {
        use crate::filters::CURRENT_YEAR;
        use crate::omdb::MediaType;
        use crate::{Filters, RuntimeConfig, Year};

        #[test]
        fn media_type() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-t",
                    "series",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    types: MediaType::SERIES,
                    years: None,
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-t",
                    "Movie",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    types: MediaType::MOVIE,
                    ..Default::default()
                }
            );
        }

        #[test]
        fn year() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    years: Some(Year(1980..=1980)),
                    ..Default::default()
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-2010",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    years: Some(Year(1980..=2010)),
                    ..Default::default()
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    years: Some(Year(1980..=*CURRENT_YEAR)),
                    ..Default::default()
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "-2010",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    years: Some(Year(0..=2010)),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn year_inverted() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "2010-1980",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    years: Some(Year(1980..=2010)),
                    ..Default::default()
                }
            );
        }

        #[test]
        fn mixed() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-2010",
                    "-t",
                    "Movies",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    types: MediaType::MOVIE,
                    years: Some(Year(1980..=2010)),
                }
            );
        }
    }

    mod filtering {
        use crate::omdb::{MediaType, SearchResult};
        use crate::{Filters, Year};
        use once_cell::sync::Lazy;

        const TEST_DATA_SIZE: usize = 6;

        static SEARCH_RESULTS: Lazy<[SearchResult; TEST_DATA_SIZE]> =
            Lazy::new(|| {
                [
                    SearchResult {
                        title: "Kingsman: The Golden Circle".into(),
                        imdb_id: "tt4649466".into(),
                        media_type: MediaType::MOVIE,
                        year: Year(2017..=2017),
                    },
                    SearchResult {
                        title: "King's Man".into(),
                        imdb_id: "tt1582211".into(),
                        media_type: MediaType::MOVIE,
                        year: Year(2010..=2010),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0405676".into(),
                        media_type: MediaType::MOVIE,
                        year: Year(2006..=2006),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0041113".into(),
                        media_type: MediaType::MOVIE,
                        year: Year(1949..=1949),
                    },
                    SearchResult {
                        title: "Black Mirror".into(),
                        imdb_id: "tt2085059".into(),
                        media_type: MediaType::SERIES,
                        year: Year(2016..=2021),
                    },
                    SearchResult {
                        title: "Seinfeld".into(),
                        imdb_id: "tt0098904".into(),
                        media_type: MediaType::SERIES,
                        year: Year(1989..=1998),
                    },
                ]
            });

        fn get_outcomes(filters: &Filters) -> Vec<bool> {
            SEARCH_RESULTS
                .iter()
                .map(|sr| {
                    let ans = filters.allows(sr);
                    println!("Do {filters:?} allow {sr}? {ans}");
                    ans
                })
                .collect()
        }

        #[test]
        fn unfiltered() {
            let default = Filters::default();
            assert_eq!(&get_outcomes(&default), &[true; TEST_DATA_SIZE]);
        }

        #[test]
        fn media_type_single() {
            let test = Filters {
                types: MediaType::MOVIE,
                years: None,
            };
            let results = [true, true, true, true, false, false];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                types: MediaType::SERIES,
                years: None,
            };
            let results = [false, false, false, false, true, true];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn years() {
            let test = Filters {
                years: Some(Year(2020..=2021)),
                ..Default::default()
            };
            let results = [false, false, false, false, true, false];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                years: Some(Year(1950..=2010)),
                ..Default::default()
            };
            let results = [false, true, true, false, false, true];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn mixed() {
            let test = Filters {
                types: MediaType::MOVIE,
                years: Some(Year(1950..=2010)),
            };
            let results = [false, true, true, false, false, false];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                types: MediaType::SERIES,
                years: Some(Year(2010..=2021)),
            };
            let results = [false, false, false, false, true, false];
            assert_eq!(&get_outcomes(&test), &results);
        }
    }
}

#[cfg(test)]
mod year_unit_tests {
    use super::Year;
    use super::CURRENT_YEAR;
    use lazy_static::lazy_static;
    use std::ops::RangeInclusive;
    use std::str::FromStr;

    lazy_static! {
        static ref STR_INPUTS: Vec<&'static str> = vec![
            "1999",
            "-1999",
            "1999–",
            "1920-1925",
            "1000-800",
            "2020–2021",
        ];
        static ref YEARS: Vec<RangeInclusive<u16>> = vec![
            1999..=1999,
            0..=1999,
            1999..=*CURRENT_YEAR,
            1920..=1925,
            800..=1000,
            2020..=2021,
        ];
    }

    #[test]
    fn from_str() {
        STR_INPUTS
            .iter()
            .map(|s| Year::from_str(s).expect("Year should have parsed"))
            .zip(YEARS.iter())
            .for_each(|(a, b)| assert_eq!(a.0, *b));
    }

    #[test]
    fn from_str_invalid() {
        Year::from_str("-").unwrap_err();
    }
}
