use crate::RunError::InvalidYearRange;
use crate::{Result, SearchResult, Year};
use clap::ArgMatches;
use std::str::FromStr;

#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Filters {
    genres: Vec<String>,
    years: Option<Year>,
}

impl Filters {
    pub fn new(clap_matches: &ArgMatches) -> Result<Self> {
        let mut genres = Vec::new();
        if let Some(vs) = clap_matches.values_of("filter_genre") {
            vs.for_each(|s| genres.push(s.into()));
        }

        let years = match clap_matches.value_of("filter_year") {
            Some(year_str) => Some(Year::from_str(year_str).map_err(|e| InvalidYearRange(e))?),
            None => None,
        };

        Ok(Filters { genres, years })
    }

    pub fn allows(&self, search_result: &SearchResult) -> bool {
        let year_matches = match (&self.years, search_result.year) {
            (Some(Year::Single(a)), Some(b)) => a == &b,
            (Some(Year::Range(range)), Some(y)) => range.contains(&y),
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

#[cfg(test)]
mod unit_tests {
    mod creation {
        use crate::{Filters, RuntimeConfig, Year::*};

        #[test]
        fn genre() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-g", "TV Episode"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec!["TV Episode".into()],
                    years: None,
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-g", "TV Episode", "Movie"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec!["TV Episode".into(), "Movie".into()],
                    years: None,
                }
            );
        }

        #[test]
        fn year() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-y", "1980"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec![],
                    years: Some(Single(1980)),
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-y", "1980-2010"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec![],
                    years: Some(Range(1980..=2010)),
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-y", "1980-"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec![],
                    years: Some(Range(1980..=u16::MAX)),
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-y", "-2010"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec![],
                    years: Some(Range(u16::MIN..=2010)),
                }
            );
        }

        #[test]
        fn year_inverted() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-y", "2010-1980"])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec![],
                    years: Some(Range(1980..=2010)),
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
                    "Video",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: vec!["Movie".into(), "Video".into()],
                    years: Some(Range(1980..=2010)),
                }
            );
        }
    }

    mod filtering {
        use crate::{Filters, SearchResult, Year::*};
        use lazy_regex::Lazy;

        static SEARCH_RESULTS: Lazy<[SearchResult; 10]> = Lazy::new(|| {
            [
                SearchResult {
                    name: "Kingsman: The Secret Service".into(),
                    id: "tt2802144".into(),
                    genre: "Movie".into(),
                    year: Some(2014),
                },
                SearchResult {
                    name: "The King's Man".into(),
                    id: "tt6856242".into(),
                    genre: "Movie".into(),
                    year: Some(2021),
                },
                SearchResult {
                    name: "Kingsman: The Golden Circle".into(),
                    id: "tt4649466".into(),
                    genre: "Movie".into(),
                    year: Some(2017),
                },
                SearchResult {
                    name: "Kingsman: The Secret Service Revealed".into(),
                    id: "tt5026378".into(),
                    genre: "Video".into(),
                    year: Some(2015),
                },
                SearchResult {
                    name: "Kingsman: Inside the Golden Circle".into(),
                    id: "tt7959890".into(),
                    genre: "Video".into(),
                    year: Some(2017),
                },
                SearchResult {
                    name: "Kingsman: Bespoke Lessons for Gentlemen Spies".into(),
                    id: "tt6597836".into(),
                    genre: "TV Series".into(),
                    year: Some(2015),
                },
                SearchResult {
                    name: "King's Man".into(),
                    id: "tt1582211".into(),
                    genre: "Movie".into(),
                    year: Some(2010),
                },
                SearchResult {
                    name: "All the King's Men".into(),
                    id: "tt0405676".into(),
                    genre: "Movie".into(),
                    year: Some(2006),
                },
                SearchResult {
                    name: "The Kingsman".into(),
                    id: "tt13332408".into(),
                    genre: "TV Episode".into(),
                    year: Some(2020),
                },
                SearchResult {
                    name: "All the King's Men".into(),
                    id: "tt0041113".into(),
                    genre: "Movie".into(),
                    year: Some(1949),
                },
            ]
        });

        fn get_outcomes(filters: &Filters) -> Vec<bool> {
            SEARCH_RESULTS.iter().map(|sr| filters.allows(sr)).collect()
        }

        #[test]
        fn unfiltered() {
            let empty = Filters {
                genres: vec![],
                years: None,
            };
            assert_eq!(&get_outcomes(&empty), &[true; 10]);

            let default = Filters::default();
            assert_eq!(&get_outcomes(&default), &[true; 10]);
        }

        #[test]
        fn genre_single() {
            let test = Filters {
                genres: vec!["Movie".into()],
                years: None,
            };
            let results = [
                true, true, true, false, false, false, true, true, false, true,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: vec!["Video".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn genre_multiple() {
            let test = Filters {
                genres: vec!["Movie".into(), "Video".into()],
                years: None,
            };
            let results = [true, true, true, true, true, false, true, true, false, true];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: vec!["Video".into(), "TV Episode".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, true, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn genre_case_insensitive() {
            let test = Filters {
                genres: vec!["movie".into()],
                years: None,
            };
            let results = [
                true, true, true, false, false, false, true, true, false, true,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: vec!["video".into()],
                years: None,
            };
            let results = [
                false, false, false, true, true, false, false, false, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn years() {
            let test = Filters {
                genres: vec![],
                years: Some(Range(2020..=u16::MAX)),
            };
            let results = [
                false, true, false, false, false, false, false, false, true, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: vec![],
                years: Some(Range(1950..=2010)),
            };
            let results = [
                false, false, false, false, false, false, true, true, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }

        #[test]
        fn mixed() {
            let test = Filters {
                genres: vec!["Movie".into()],
                years: Some(Range(1950..=2010)),
            };
            let results = [
                false, false, false, false, false, false, true, true, false, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);

            let test = Filters {
                genres: vec!["Movie".into(), "TV Episode".into()],
                years: Some(Range(2010..=u16::MAX)),
            };
            let results = [
                true, true, true, false, false, false, true, false, true, false,
            ];
            assert_eq!(&get_outcomes(&test), &results);
        }
    }
}
