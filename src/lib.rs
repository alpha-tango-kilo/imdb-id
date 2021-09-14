mod clap_wrap;
mod errors;
mod filters;
pub mod omdb;
mod user_input;

pub use clap_wrap::*;
pub use errors::*;
pub use filters::*;
pub use reqwest::blocking as reqwest;
pub use user_input::{get_api_key, Pager};

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::iter::FromIterator;
use std::num::ParseIntError;
use std::ops::RangeInclusive;
use std::str::FromStr;
// Has to use different name or re-export of errors::Result wouldn't work
use smallvec::SmallVec;
use std::result::Result as StdResult;

#[derive(Debug, Clone, Serialize)]
// Serialise using Display impl by using it in impl Into<String>
#[serde(into = "String")]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum Year {
    Single(u16),
    Range(RangeInclusive<u16>),
    // TODO: add Start/End variants?
}

impl Year {
    const SEPARATORS: [char; 2] = ['-', '–'];
}

impl FromStr for Year {
    type Err = ParseIntError;

    // WARNING: not all separators are one byte, this must not be assumed!
    fn from_str(year_str: &str) -> StdResult<Self, Self::Err> {
        use std::mem;

        let mut start = u16::MIN;
        let mut end = u16::MAX;
        // e.g. -2021
        if year_str.starts_with(&Year::SEPARATORS[..]) {
            end = year_str.chars().skip(1).collect::<String>().parse()?;
            // e.g. 1999-
        } else if year_str.ends_with(&Year::SEPARATORS[..]) {
            // Get list of chars
            let chars = year_str.chars().collect::<SmallVec<[char; 5]>>();
            // Remove last one (the dash)
            let chars = &chars[..chars.len() - 1];
            // Create String from iterator so we can parse
            start = String::from_iter(chars).parse()?;
        } else {
            match year_str.split_once(&Year::SEPARATORS[..]) {
                // e.g. 1999 - 2021
                Some((s, e)) => {
                    start = s.parse()?;
                    end = e.parse()?;
                    if start > end {
                        // User is rather stupid, let's save them
                        mem::swap(&mut start, &mut end);
                    }
                }
                // e.g. 2010
                None => {
                    let n = year_str.parse()?;
                    return Ok(Year::Single(n));
                }
            }
        }
        Ok(Year::Range(start..=end))
    }
}

impl<'de> Deserialize<'de> for Year {
    fn deserialize<D>(d: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(d)?;
        Year::from_str(&s)
            .map_err(|e| D::Error::custom(format!("Could not parse field as year ({:?})", e)))
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
            Single(y) => write!(f, "{}", y),
            Range(r) => write!(f, "{}-{}", r.start(), r.end()),
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

    const EXPECTED: [Year; 6] = [
        Single(1999),
        Range(u16::MIN..=1999),
        Range(1999..=u16::MAX),
        Range(1920..=1925),
        Range(800..=1000),
        Range(2020..=2021),
    ];

    #[test]
    fn from_str() {
        STR_INPUTS
            .iter()
            .map(|s| Year::from_str(s).expect("Year should have parsed"))
            .zip(EXPECTED.iter())
            .for_each(|(a, b)| assert_eq!(a, *b));
    }
}
