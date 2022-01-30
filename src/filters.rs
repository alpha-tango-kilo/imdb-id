use crate::omdb::SearchResult;
use crate::{Result, RunError, YearParseError};
use clap::ArgMatches;
use lazy_static::lazy_static;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;
use Genre::*;

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
    pub genres: SmallVec<[Genre; 3]>,
    pub years: Option<Year>,
}

impl Filters {
    pub fn new(clap_matches: &ArgMatches) -> Result<Self> {
        let mut genres = SmallVec::new();
        if let Some(vs) = clap_matches.values_of("filter_genre") {
            for v in vs {
                genres.push(Genre::from_str(v)?);
            }
        }

        let years = match clap_matches.value_of("filter_year") {
            Some(year_str) => Year::from_str(year_str)?.into(),
            None => None,
        };

        Ok(Filters { genres, years })
    }

    pub fn allows(&self, search_result: &SearchResult) -> bool {
        let year_matches = match &self.years {
            Some(year) => year.contains(&search_result.year),
            None => true,
        };

        let genre_matches = self.genres.is_empty()
            || self.genres.iter().any(|allowed_genre| {
                allowed_genre.eq(search_result.media_type.as_str())
            });
        //println!("{:?}\n^ year matches: {}, genre matches: {}", search_result, year_matches, genre_matches);
        year_matches && genre_matches
    }
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            genres: smallvec![],
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
                    u16::from_str(start_str)?
                } else {
                    0
                };
                let mut end = if !end_str.is_empty() {
                    u16::from_str(end_str)?
                } else if start_str.is_empty() {
                    return Err(NoYearsSpecified);
                } else {
                    *CURRENT_YEAR
                };
                // Save the user from their silliness
                if end < start {
                    use std::mem;
                    mem::swap(&mut start, &mut end);
                }
                Ok(Year(start..=end))
            }
            None => {
                // Should be just a year we can parse
                let year = u16::from_str(year_str)?;
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
            D::Error::custom(format!("could not parse field as year ({:?})", e))
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

// These are the OMDb API supported genres to filter by
// Serialize and Deserialize and implemented by hand
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Genre {
    Movie,
    Series,
    Episode,
    Other(String),
}

impl Genre {
    fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl AsRef<str> for Genre {
    fn as_ref(&self) -> &str {
        match self {
            Movie => "movie",
            Series => "series",
            Episode => "episode",
            Other(s) => s,
        }
    }
}

impl FromStr for Genre {
    type Err = RunError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "movie" => Ok(Movie),
            "series" => Ok(Series),
            "episode" => Ok(Episode),
            _ => Err(RunError::InvalidGenre(s.to_owned())),
        }
    }
}

impl From<&str> for Genre {
    fn from(s: &str) -> Self {
        Self::from_str(s).unwrap_or_else(|_| Other(s.to_owned()))
    }
}

impl PartialEq<str> for Genre {
    fn eq(&self, other: &str) -> bool {
        other.eq_ignore_ascii_case(self.as_str())
    }
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

// Serialize with Genre.as_str
impl Serialize for Genre {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// Deserialize with From<str> for Genre
impl<'de> Deserialize<'de> for Genre {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|s| Genre::from(s.as_str()))
    }
}

#[cfg(test)]
mod filters_unit_tests {
    mod creation {
        use crate::filters::CURRENT_YEAR;
        use crate::Genre::*;
        use crate::{Filters, RuntimeConfig, Year};
        use smallvec::smallvec;

        #[test]
        fn genre() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-g",
                    "episode",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec![Episode],
                    years: None,
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-g",
                    "Episode",
                    "Movie",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec![Episode, Movie],
                    years: None,
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
                    genres: smallvec![],
                    years: Some(Year(1980..=1980)),
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
                    genres: smallvec![],
                    years: Some(Year(1980..=2010)),
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
                    genres: smallvec![],
                    years: Some(Year(1980..=*CURRENT_YEAR)),
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
                    genres: smallvec![],
                    years: Some(Year(0..=2010)),
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
                    genres: smallvec![],
                    years: Some(Year(1980..=2010)),
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
                    "-g",
                    "Movie",
                    "episode",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec![Movie, Episode],
                    years: Some(Year(1980..=2010)),
                }
            );
        }
    }

    mod filtering {
        use crate::omdb::SearchResult;
        use crate::Genre::*;
        use crate::{Filters, Year};
        use once_cell::sync::Lazy;
        use smallvec::smallvec;

        const TEST_DATA_SIZE: usize = 12;

        static SEARCH_RESULTS: Lazy<[SearchResult; TEST_DATA_SIZE]> =
            Lazy::new(|| {
                [
                    SearchResult {
                        title: "Kingsman: The Secret Service".into(),
                        imdb_id: "tt2802144".into(),
                        media_type: Movie,
                        year: Year(2014..=2014),
                    },
                    SearchResult {
                        title: "The King's Man".into(),
                        imdb_id: "tt6856242".into(),
                        media_type: Movie,
                        year: Year(2021..=2021),
                    },
                    SearchResult {
                        title: "Kingsman: The Golden Circle".into(),
                        imdb_id: "tt4649466".into(),
                        media_type: Movie,
                        year: Year(2017..=2017),
                    },
                    SearchResult {
                        title: "Kingsman: The Secret Service Revealed".into(),
                        imdb_id: "tt5026378".into(),
                        media_type: "Video".into(),
                        year: Year(2015..=2015),
                    },
                    SearchResult {
                        title: "Kingsman: Inside the Golden Circle".into(),
                        imdb_id: "tt7959890".into(),
                        media_type: "Video".into(),
                        year: Year(2017..=2017),
                    },
                    SearchResult {
                        title: "Kingsman: Bespoke Lessons for Gentlemen Spies"
                            .into(),
                        imdb_id: "tt6597836".into(),
                        media_type: "TV Series".into(),
                        year: Year(2015..=2015),
                    },
                    SearchResult {
                        title: "King's Man".into(),
                        imdb_id: "tt1582211".into(),
                        media_type: Movie,
                        year: Year(2010..=2010),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0405676".into(),
                        media_type: Movie,
                        year: Year(2006..=2006),
                    },
                    SearchResult {
                        title: "The Kingsman".into(),
                        imdb_id: "tt13332408".into(),
                        media_type: "TV Episode".into(),
                        year: Year(2020..=2020),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0041113".into(),
                        media_type: Movie,
                        year: Year(1949..=1949),
                    },
                    SearchResult {
                        title: "Black Mirror".into(),
                        imdb_id: "tt2085059".into(),
                        media_type: Series,
                        year: Year(2016..=2021),
                    },
                    SearchResult {
                        title: "Seinfeld".into(),
                        imdb_id: "tt0098904".into(),
                        media_type: Series,
                        year: Year(1989..=1998),
                    },
                ]
            });

        fn get_outcomes(filters: &Filters) -> Vec<bool> {
            SEARCH_RESULTS
                .iter()
                .map(|sr| {
                    let ans = filters.allows(sr);
                    println!("Do {:?} allow {}? {}", filters, sr, ans);
                    ans
                })
                .collect()
        }

        #[test]
        fn unfiltered() {
            let empty = Filters {
                genres: smallvec![],
                years: None,
            };
            assert_eq!(&get_outcomes(&empty), &[true; TEST_DATA_SIZE]);

            let default = Filters::default();
            assert_eq!(&get_outcomes(&default), &[true; TEST_DATA_SIZE]);
        }

        #[test]
        fn genre_single() {
            let test = Filters {
                genres: smallvec![Movie],
                years: None,
            };
            let results = [
                true, true, true, false, false, false, true, true, false, true,
                false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec!["Video".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, false,
                false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn genre_multiple() {
            let test = Filters {
                genres: smallvec![Movie, "Video".into()],
                years: None,
            };
            let results = [
                true, true, true, true, true, false, true, true, false, true,
                false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec!["Video".into(), "TV Episode".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, true,
                false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn genre_case_insensitive() {
            let test = Filters {
                genres: smallvec!["movie".into()],
                years: None,
            };
            let results = [
                true, true, true, false, false, false, true, true, false, true,
                false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec!["video".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, false,
                false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn years() {
            let test = Filters {
                genres: smallvec![],
                years: Some(Year(2020..=2021)),
            };
            let results = [
                false, true, false, false, false, false, false, false, true,
                false, true, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec![],
                years: Some(Year(1950..=2010)),
            };
            let results = [
                false, false, false, false, false, false, true, true, false,
                false, false, true,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn mixed() {
            let test = Filters {
                genres: smallvec![Movie],
                years: Some(Year(1950..=2010)),
            };
            let results = [
                false, false, false, false, false, false, true, true, false,
                false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec![Movie, "TV Episode".into()],
                years: Some(Year(2010..=2021)),
            };
            let results = [
                true, true, true, false, false, false, true, false, true,
                false, false, false,
            ];
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

    const STR_INPUTS: [&str; 7] = [
        "1999",
        "-1999",
        "1999–",
        "1920-1925",
        "1000-800",
        "2020–2021",
        "2048-",
    ];

    lazy_static! {
        static ref YEARS: [RangeInclusive<u16>; 7] = [
            1999..=1999,
            0..=1999,
            1999..=*CURRENT_YEAR,
            1920..=1925,
            800..=1000,
            2020..=2021,
            *CURRENT_YEAR..=2048,
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
