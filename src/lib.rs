mod clap_wrap;
mod errors;
mod filters;
pub mod omdb;
mod persistent;
mod user_input;

pub use clap_wrap::*;
pub use errors::*;
pub use filters::*;
pub use persistent::*;
pub use user_input::{choose_result_from, get_api_key};

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::iter::FromIterator;
use std::num::ParseIntError;
use std::str::FromStr;
// Has to use different name or re-export of errors::Result wouldn't work
use smallvec::SmallVec;
use Year::*;
use Genre::*;

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
            _ => Err(RunError::InvalidGenre(s.to_owned()))
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
        where S: Serializer
    {
        serializer.serialize_str(self.as_str())
    }
}

// Deserialize with From<str> for Genre
impl<'de> Deserialize<'de> for Genre {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        String::deserialize(deserializer).map(|s| Genre::from(s.as_str()))
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
