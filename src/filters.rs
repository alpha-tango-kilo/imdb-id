use crate::omdb::SearchResult;
use crate::RunError::InvalidYearRange;
use crate::{Result, Year};
use clap::ArgMatches;
use smallvec::{smallvec, SmallVec};
use std::str::FromStr;

#[derive(Debug)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub struct Filters {
    genres: SmallVec<[String; 3]>,
    years: Option<Year>,
}

impl Filters {
    pub fn new(clap_matches: &ArgMatches) -> Result<Self> {
        let mut genres = SmallVec::new();
        if let Some(vs) = clap_matches.values_of("filter_genre") {
            vs.for_each(|s| genres.push(s.into()));
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
                search_result.media_type.eq_ignore_ascii_case(allowed_genre)
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

#[cfg(test)]
mod unit_tests {
    mod creation {
        use crate::{Filters, RuntimeConfig, Year, Year::*};
        use smallvec::smallvec;

        #[test]
        fn genre() {
            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-g",
                    "TV Episode",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec!["TV Episode".into()],
                    years: None,
                }
            );

            let clap = RuntimeConfig::create_clap_app();
            let clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-g",
                    "TV Episode",
                    "Movie",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec!["TV Episode".into(), "Movie".into()],
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
                    "Video",
                ])
                .unwrap();
            let filters = Filters::new(&clap_matches).unwrap();
            assert_eq!(
                filters,
                Filters {
                    genres: smallvec!["Movie".into(), "Video".into()],
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
                        media_type: "Movie".into(),
                        year: Year::Single(2014),
                    },
                    SearchResult {
                        title: "The King's Man".into(),
                        imdb_id: "tt6856242".into(),
                        media_type: "Movie".into(),
                        year: Year::Single(2021),
                    },
                    SearchResult {
                        title: "Kingsman: The Golden Circle".into(),
                        imdb_id: "tt4649466".into(),
                        media_type: "Movie".into(),
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
                        media_type: "Movie".into(),
                        year: Year::Single(2010),
                    },
                    SearchResult {
                        title: "All the King's Men".into(),
                        imdb_id: "tt0405676".into(),
                        media_type: "Movie".into(),
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
                        media_type: "Movie".into(),
                        year: Year::Single(1949),
                    },
                    SearchResult {
                        title: "Black Mirror".into(),
                        imdb_id: "tt2085059".into(),
                        media_type: "Series".into(),
                        year: Year::Range {
                            start: Some(2016),
                            end: None,
                        },
                    },
                    SearchResult {
                        title: "Seinfeld".into(),
                        imdb_id: "tt0098904".into(),
                        media_type: "Series".into(),
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
                genres: smallvec!["Movie".into()],
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
                genres: smallvec!["Movie".into(), "Video".into()],
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
                genres: smallvec!["Movie".into()],
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
                genres: smallvec!["Movie".into(), "TV Episode".into()],
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
