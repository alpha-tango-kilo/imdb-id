use crate::omdb::SearchResult;
use crate::RunError::InvalidYearRange;
use crate::{Result, RunError};
use clap::ArgMatches;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smallvec::{smallvec, SmallVec};
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;
use Genre::*;
use Year::*;

#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Filters {
    genres: SmallVec<[Genre; 3]>,
    years: Option<Year>,
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
            Some(year_str) => {
                Some(Year::from_str(year_str).map_err(InvalidYearRange)?)
            }
            None => None,
        };

        Ok(Filters { genres, years })
    }

    pub fn allows(&self, search_result: &SearchResult) -> bool {
        let year_matches = match &self.years {
            Some(year) => match (year, &search_result.year) {
                (Year::Single(a), Year::Single(b)) => a == b,
                (
                    Year::Range { start: None, .. },
                    Year::Range { start: None, .. },
                ) => true,
                (
                    Year::Range { end: None, .. },
                    Year::Range { end: None, .. },
                ) => true,
                (Year::Range { .. }, Year::Range { start, end }) => {
                    start.map_or(false, |s| year.contains(s))
                        || end.map_or(false, |s| year.contains(s))
                }
                (Year::Single(a), Year::Range { .. }) => {
                    search_result.year.contains(*a)
                }
                (Year::Range { .. }, Year::Single(b)) => year.contains(*b),
            },
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

#[derive(Debug, Copy, Clone, Serialize)]
// Serialise using Display impl by using it in impl Into<String>
#[serde(into = "String")]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Year {
    Single(u16),
    // start and end should never both be None
    Range {
        start: Option<u16>,
        end: Option<u16>,
    },
}

impl Year {
    const SEPARATORS: [char; 2] = ['-', '–'];

    pub fn contains(&self, year: u16) -> bool {
        match *self {
            Single(n) => n == year,
            Range { start, end } => {
                start.map_or(true, |n| year >= n)
                    && end.map_or(true, |n| year <= n)
            }
        }
    }
}

impl FromStr for Year {
    type Err = ParseIntError;

    // WARNING: not all separators are one byte, this must not be assumed!
    fn from_str(year_str: &str) -> Result<Self, Self::Err> {
        use std::mem;
        // e.g. -2021
        if year_str.starts_with(&Year::SEPARATORS[..]) {
            let end = year_str
                .chars()
                .skip(1)
                .collect::<String>()
                .parse::<u16>()?
                .into();
            Ok(Year::Range { start: None, end })
            // e.g. 1999-
        } else if year_str.ends_with(&Year::SEPARATORS[..]) {
            // Get list of chars
            let chars = year_str.chars().collect::<SmallVec<[char; 5]>>();
            // Remove the dash and create String from slice so we can parse
            let start = String::from_iter(&chars[..chars.len() - 1])
                .parse::<u16>()?
                .into();
            Ok(Year::Range { start, end: None })
        } else {
            match year_str.split_once(&Year::SEPARATORS[..]) {
                // e.g. 1999 - 2021
                Some((s, e)) => {
                    let mut start = s.parse::<u16>()?;
                    let mut end = e.parse::<u16>()?;
                    if start > end {
                        // User is rather stupid, let's save them
                        mem::swap(&mut start, &mut end);
                    }
                    Ok(Year::Range {
                        start: start.into(),
                        end: end.into(),
                    })
                }
                // e.g. 2010
                None => {
                    let n = year_str.parse()?;
                    Ok(Year::Single(n))
                }
            }
        }
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

// Used with serialisation
impl From<Year> for String {
    fn from(year: Year) -> Self {
        year.to_string()
    }
}

impl fmt::Display for Year {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Year::*;
        match self {
            Single(y) => write!(f, "{y}"),
            Range { start, end } => {
                if let Some(n) = start {
                    write!(f, "{n}")?;
                }
                write!(f, "-")?;
                if let Some(n) = end {
                    write!(f, "{n}")?;
                }
                Ok(())
            }
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
        use crate::Genre::*;
        use crate::{Filters, RuntimeConfig, Year, Year::*};
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
                    years: Some(Single(1980)),
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
                    years: Some(Year::Range {
                        start: Some(1980),
                        end: Some(2010),
                    }),
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
                    years: Some(Year::Range {
                        start: Some(1980),
                        end: None,
                    }),
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
                    years: Some(Year::Range {
                        start: None,
                        end: Some(2010),
                    }),
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
                    years: Some(Year::Range {
                        start: Some(1980),
                        end: Some(2010),
                    }),
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
                    years: Some(Year::Range {
                        start: Some(1980),
                        end: Some(2010),
                    }),
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
                        year: Year::Single(2014),
                    },
                    SearchResult {
                        title: "The King's Man".into(),
                        imdb_id: "tt6856242".into(),
                        media_type: Movie,
                        year: Year::Single(2021),
                    },
                    SearchResult {
                        title: "Kingsman: The Golden Circle".into(),
                        imdb_id: "tt4649466".into(),
                        media_type: Movie,
                        year: Year::Single(2017),
                    },
                    SearchResult {
                        title: "Kingsman: The Secret Service Revealed".into(),
                        imdb_id: "tt5026378".into(),
                        media_type: "Video".into(),
                        year: Year::Single(2015),
                    },
                    SearchResult {
                        title: "Kingsman: Inside the Golden Circle".into(),
                        imdb_id: "tt7959890".into(),
                        media_type: "Video".into(),
                        year: Year::Single(2017),
                    },
                    SearchResult {
                        title: "Kingsman: Bespoke Lessons for Gentlemen Spies"
                            .into(),
                        imdb_id: "tt6597836".into(),
                        media_type: "TV Series".into(),
                        year: Year::Single(2015),
                    },
                    SearchResult {
                        title: "King's Man".into(),
                        imdb_id: "tt1582211".into(),
                        media_type: Movie,
                        year: Year::Single(2010),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0405676".into(),
                        media_type: Movie,
                        year: Year::Single(2006),
                    },
                    SearchResult {
                        title: "The Kingsman".into(),
                        imdb_id: "tt13332408".into(),
                        media_type: "TV Episode".into(),
                        year: Year::Single(2020),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0041113".into(),
                        media_type: Movie,
                        year: Year::Single(1949),
                    },
                    SearchResult {
                        title: "Black Mirror".into(),
                        imdb_id: "tt2085059".into(),
                        media_type: Series,
                        year: Year::Range {
                            start: Some(2016),
                            end: None,
                        },
                    },
                    SearchResult {
                        title: "Seinfeld".into(),
                        imdb_id: "tt0098904".into(),
                        media_type: Series,
                        year: Year::Range {
                            start: Some(1989),
                            end: Some(1998),
                        },
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
                years: Some(Year::Range {
                    start: Some(2020),
                    end: None,
                }),
            };
            let results = [
                false, true, false, false, false, false, false, false, true,
                false, true, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec![],
                years: Some(Year::Range {
                    start: Some(1950),
                    end: Some(2010),
                }),
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
                years: Some(Year::Range {
                    start: Some(1950),
                    end: Some(2010),
                }),
            };
            let results = [
                false, false, false, false, false, false, true, true, false,
                false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: smallvec![Movie, "TV Episode".into()],
                years: Some(Year::Range {
                    start: Some(2010),
                    end: None,
                }),
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
    use super::Year::{self, *};
    use std::str::FromStr;

    const STR_INPUTS: [&str; 6] = [
        "1999",
        "-1999",
        "1999–",
        "1920-1925",
        "1000-800",
        "2020–2021",
    ];

    const YEARS: [Year; 6] = [
        Single(1999),
        Range {
            start: None,
            end: Some(1999),
        },
        Range {
            start: Some(1999),
            end: None,
        },
        Range {
            start: Some(1920),
            end: Some(1925),
        },
        Range {
            start: Some(800),
            end: Some(1000),
        },
        Range {
            start: Some(2020),
            end: Some(2021),
        },
    ];

    #[test]
    fn from_str() {
        STR_INPUTS
            .iter()
            .map(|s| Year::from_str(s).expect("Year should have parsed"))
            .zip(YEARS.iter())
            .for_each(|(a, b)| assert_eq!(a, *b));
    }

    #[test]
    fn from_str_invalid() {
        Year::from_str("-").unwrap_err();
    }

    #[test]
    fn contain() {
        use Year::*;

        YEARS.iter().for_each(|year| match *year {
            Single(y) => {
                assert!(year.contains(y));
                assert!(!year.contains(y + 1));
                assert!(!year.contains(y - 1));
            }
            Range {
                start: Some(s),
                end: Some(e),
            } => {
                (s..e).into_iter().for_each(|n| assert!(year.contains(n)));
                assert!(!year.contains(s - 1));
                assert!(!year.contains(e + 1));
            }
            Range {
                start: None,
                end: Some(e),
            } => {
                (0..e).into_iter().for_each(|n| assert!(year.contains(n)));
                assert!(!year.contains(e + 1));
            }
            Range {
                start: Some(s),
                end: None,
            } => {
                (s..u16::MAX)
                    .into_iter()
                    .for_each(|n| assert!(year.contains(n)));
                assert!(!year.contains(s - 1));
            }
            _ => {
                unreachable!("Invalid test - range with start and end as None")
            }
        })
    }
}
