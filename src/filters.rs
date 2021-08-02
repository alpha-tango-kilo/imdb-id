use crate::RunError::InvalidYearRange;
use crate::{Result, SearchResult};
use clap::ArgMatches;
use std::ops::RangeInclusive;
use std::mem;

#[derive(Debug)]
pub struct Filters {
    genres: Vec<String>,
    years: Option<RangeInclusive<u16>>,
}

impl Filters {
    pub fn new(clap_matches: &ArgMatches) -> Result<Self> {
        let mut genres = Vec::new();
        if let Some(vs) = clap_matches.values_of("filter_genre") {
            vs.for_each(|s| genres.push(s.into()));
        }

        let years = match clap_matches.value_of("filter_year") {
            Some(year_str) => Some(Filters::year_range_parse(year_str)?),
            None => None,
        };

        Ok(Filters { genres, years })
    }

    fn year_range_parse(year_str: &str) -> Result<RangeInclusive<u16>> {
        let mut start = u16::MIN;
        let mut end = u16::MAX;
        // e.g. -2021
        if year_str.starts_with('-') {
            end = year_str[1..].parse().map_err(|e| InvalidYearRange(e))?;
        // e.g. 1999-
        } else if year_str.ends_with('-') {
            start = year_str[..year_str.len() - 1]
                .parse()
                .map_err(|e| InvalidYearRange(e))?;
        } else {
            match year_str.split_once('-') {
                // e.g. 1999 - 2021
                Some((s, e)) => {
                    start = s.parse().map_err(|e| InvalidYearRange(e))?;
                    end = e.parse().map_err(|e| InvalidYearRange(e))?;
                    if start > end {
                        // User is rather stupid, let's save them
                        mem::swap(&mut start, &mut end);
                    }
                }
                // e.g. 2010
                None => {
                    let n = year_str.parse().map_err(|e| InvalidYearRange(e))?;
                    start = n;
                    end = n;
                }
            }
        }
        Ok(start..=end)
    }

    pub fn allows(&self, search_result: &SearchResult) -> bool {
        let year_matches = match (&self.years, search_result.year) {
            (Some(range), Some(y)) => range.contains(&y),
            _ => true,
        };
        let genre_matches = self.genres.is_empty()
            || self
                .genres
                .iter()
                .any(|allowed_genre| search_result.genre.eq_ignore_ascii_case(allowed_genre));
        //println!("{:?}\n^ year matches: {}, genre matches: {}", search_result, year_matches, genre_matches);
        year_matches && genre_matches
    }
}

impl Default for Filters {
    fn default() -> Self {
        Filters {
            genres: vec![],
            years: None,
        }
    }
}
