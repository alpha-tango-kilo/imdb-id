use crate::{user_input, Filters, Result, RunError};
#[cfg(not(test))]
use atty::Stream;
use clap::{App, AppSettings, Arg, ArgMatches};
use OutputFormat::*;

use std::convert::TryFrom;
/*
use lazy_regex::Lazy;

static SUPPORTED_FORMATS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut v = Vec::with_capacity(2);
    if cfg!(feature = "json") {
        v.push("json");
    }
    if cfg!(feature = "yaml") {
        v.push("yaml");
    }
    v
});
 */

pub struct RuntimeConfig {
    pub search_term: String,
    pub interactive: bool,
    pub number_of_results: usize,
    pub filters: Filters,
    pub format: OutputFormat,
}

impl RuntimeConfig {
    pub fn new() -> Result<Self> {
        RuntimeConfig::process_matches(&RuntimeConfig::create_clap_app().get_matches())
    }

    // public for testing purposes in filters.rs
    pub(crate) fn create_clap_app() -> clap::App<'static> {
        let mut base_args = vec![
            Arg::new("non-interactive")
                .short('n')
                .long("non-interactive")
                .about("Disables interactive features (always picks the first result)")
                .requires("search_term"),
            Arg::new("number_of_results")
                .short('r')
                .long("results")
                .about("The maximum number of results to show from IMDb")
                .takes_value(true)
                .conflicts_with("non-interactive")
                .validator(|s| s.parse::<usize>().map_err(|_| RunError::ClapNotUsize)),
            Arg::new("filter_genre")
                .short('g')
                .long("genre")
                .about("Filters results to a specific genre")
                .long_about(
                    "Filters results to a specific genre\n\
                    Can be given multiple arguments or passed multiple times, \
                    working as a chain of OR statements logically. \
                    Filters are all case insensitive\n\
                    It is STRONGLY recommended you quote genres, as most have \
                    spaces\n\
                    Examples: Movie, \"TV episode\", \"TV series\"",
                )
                .takes_value(true)
                .multiple(true),
            Arg::new("filter_year")
                .short('y')
                .long("year")
                .about("Filter results to a specific year")
                .long_about(
                    "Filters results to a specific year, or range of years\n\
                    Media which has no year specified will always be included\n\
                    Ranges are fully inclusive\n\
                    Examples: 2021, 1990-2000, 2000- (2000 onwards), \
                    -2000 (before 2000)",
                )
                .takes_value(true)
                .allow_hyphen_values(true),
            Arg::new("search_term")
                .about("The title of the movie/show you're looking for")
                .takes_value(true)
                .multiple(true),
        ];

        if cfg!(feature = "serde") {
            base_args.push(
                Arg::new("format")
                    .short('f')
                    .long("format")
                    .about("Change output format to desired standard")
                    .long_about(
                        "Change output format to desired standard\n\
                    Formats are only available if you opted-IN at installation\n\
                    All the formats imdb-id can support are: json, yaml",
                    )
                    .validator(|s| OutputFormat::try_from(s))
                    .takes_value(true),
            );
        } /* else {
              // Mimic exact behaviour of format but always error
              // Gives program consistent API
              base_args.push(
                  Arg::new("format")
                      .short('f')
                      .long("format")
                      .about("(DISABLED) Change output format to desired standard")
                      .validator(|_| -> Result<()> {
                          Err(RunError::ClapMissingFeature("'json' and/or 'yaml'"))
                      })
                      .takes_value(true),
              )
          }*/

        App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("alpha-tango-kilo <git@heyatk.com>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .setting(AppSettings::TrailingVarArg)
            .args(base_args)
    }

    fn process_matches(clap_matches: &ArgMatches) -> Result<Self> {
        let search_term = match clap_matches.values_of("search_term") {
            Some(vs) => {
                // TODO: there has to be a better way than this
                let mut search_term = String::new();
                vs.into_iter().for_each(|v| {
                    search_term.push_str(v);
                    search_term.push(' ');
                });
                search_term.trim().into()
            }
            None => {
                if cfg!(not(test)) {
                    user_input::get_search_term()?
                } else {
                    String::new()
                }
            }
        };

        // Note: atty checks are disabled for testing
        #[cfg(not(test))]
        let interactive = !clap_matches.is_present("non-interactive")
            && atty::is(Stream::Stdout)
            && atty::is(Stream::Stdin);
        #[cfg(test)]
        let interactive = !clap_matches.is_present("non-interactive");

        let number_of_results = if interactive {
            match clap_matches.value_of("number_of_results") {
                Some(n) => n.parse().unwrap(),
                None => RuntimeConfig::default().number_of_results,
            }
        } else {
            1
        };

        let format = match clap_matches.value_of("format") {
            Some(s) => OutputFormat::try_from(s)?,
            None => RuntimeConfig::default().format,
        };

        Ok(RuntimeConfig {
            search_term,
            interactive,
            number_of_results,
            filters: Filters::new(&clap_matches)?,
            format,
        })
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            search_term: String::new(),
            interactive: true,
            number_of_results: 10,
            filters: Filters::default(),
            format: Human,
        }
    }
}

#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
pub enum OutputFormat {
    Human,
    #[cfg(feature = "json")]
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
}

impl TryFrom<&str> for OutputFormat {
    type Error = RunError;

    fn try_from(s: &str) -> Result<Self> {
        let variant = match s.to_ascii_lowercase().as_str() {
            "human" | "plain" => Human,
            #[cfg(feature = "json")]
            "json" => Json,
            #[cfg(feature = "yaml")]
            "yaml" => Yaml,
            _ => return Err(RunError::ClapInvalidFormat),
        };
        Ok(variant)
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn help() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-h"])
            .unwrap_err();
        assert_eq!(m.kind, clap::ErrorKind::DisplayHelp);

        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--help"])
            .unwrap_err();
        assert_eq!(m.kind, clap::ErrorKind::DisplayHelp);
    }

    #[test]
    fn version() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-V"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::DisplayVersion);

        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--version"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::DisplayVersion);
    }

    #[test]
    fn results_short() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-r", "3", "foo"])
            .unwrap();
        assert_eq!(m.value_of("number_of_results"), Some("3"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.number_of_results, 3);
    }

    #[test]
    fn results_long() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--results", "7", "foo"])
            .unwrap();
        assert_eq!(m.value_of("number_of_results"), Some("7"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.number_of_results, 7);
    }

    #[test]
    fn results_invalid() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--results", "bar", "foo"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::ValueValidation);
    }

    #[test]
    fn non_interactive_short() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-n", "foo"])
            .unwrap();
        assert!(m.is_present("non-interactive"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert!(!config.interactive);
        assert_eq!(config.number_of_results, 1);
    }

    #[test]
    fn non_interactive_long() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--non-interactive", "foo"])
            .unwrap();
        assert!(m.is_present("non-interactive"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert!(!config.interactive);
        assert_eq!(config.number_of_results, 1);
    }

    #[test]
    fn conflicting_r_n() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--non-interactive",
                "--results",
                "5",
                "foo",
            ])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn require_search_term_if_n() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--non-interactive"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::MissingRequiredArgument)
    }

    #[test]
    fn multiple_word_search_term() {
        let clap = RuntimeConfig::create_clap_app();
        let matches = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "foo", "bar"])
            .unwrap();
        let values = matches.values_of("search_term").unwrap();
        assert_eq!(values.len(), 2);

        let config = RuntimeConfig::process_matches(&matches).unwrap();
        assert_eq!(&config.search_term, "foo bar");
    }

    #[test]
    #[cfg(not(feature = "serde"))]
    fn format_fails() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--format", "foo"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::UnknownArgument);
    }

    #[cfg(all(feature = "serde", feature = "json", feature = "yaml"))]
    mod serde {
        use super::*;

        #[test]
        fn format_short() {
            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-f", "json"])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("json"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Json);

            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-f", "yaml"])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("yaml"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }

        #[test]
        fn format_long() {
            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--format", "json"])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("json"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Json);

            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--format", "yaml"])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("yaml"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }

        #[test]
        fn invalid_format() {
            let clap = RuntimeConfig::create_clap_app();
            let err = clap
                .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--format", "foo"])
                .unwrap_err();
            assert_eq!(err.kind, clap::ErrorKind::ValueValidation);
        }
    }
}
