use crate::{user_input, ClapError, Filters, Result};
use clap::{App, AppSettings, Arg, ArgMatches};
use OutputFormat::*;

use std::convert::TryFrom;
use itertools::Itertools;
use trim_in_place::TrimInPlace;

pub struct RuntimeConfig {
    pub search_term: String,
    pub interactive: bool,
    pub number_of_results: usize,
    pub filters: Filters,
    pub format: OutputFormat,
    pub api_key: Option<String>,
}

impl RuntimeConfig {
    pub fn new() -> Result<Self> {
        RuntimeConfig::process_matches(
            &RuntimeConfig::create_clap_app().get_matches(),
        )
    }

    // public for testing purposes in filters.rs
    pub(crate) fn create_clap_app() -> clap::App<'static> {
        App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("alpha-tango-kilo <git@heyatk.com>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .setting(AppSettings::TrailingVarArg)
            .arg(
                Arg::new("non-interactive")
                    .short('n')
                    .long("non-interactive")
                    .help("Disables interactive features (always picks the first result)")
                    .requires("search_term"),
            )
            .arg(
                Arg::new("number_of_results")
                    .short('r')
                    .long("results")
                    .help("The maximum number of results to show from IMDb")
                    .takes_value(true)
                    .conflicts_with("non-interactive")
                    .validator(|s| s.parse::<usize>().map_err(|_| ClapError::NotUsize)),
            )
            .arg(
                Arg::new("filter_type")
                    .short('t')
                    .long("type")
                    .help("Filters results to a specific media type (movie or series)")
                    .takes_value(true),
            )
            .arg(
                Arg::new("filter_year")
                    .short('y')
                    .long("year")
                    .help("Filter results to a specific year")
                    .long_help(
                        "Filters results to a specific year, or range of years\n\
                    Media which has no year specified will always be included\n\
                    Ranges are fully inclusive\n\
                    Examples: 2021, 1990-2000, 2000- (2000 onwards), \
                    -2000 (before 2000)",
                    )
                    .takes_value(true)
                    .allow_hyphen_values(true),
            )
            .arg(
                Arg::new("format")
                    .short('f')
                    .long("format")
                    .help("Change output format to desired standard")
                    .long_help(
                        "Change output format to desired standard\n\
                    Formats are only available if you opted-IN at installation\n\
                    All the formats imdb-id can support are: json, yaml",
                    )
                    .validator(|s| OutputFormat::try_from(s))
                    .takes_value(true),
            )
            .arg(
                Arg::new("search_term")
                    .help("The title of the movie/show you're looking for")
                    .takes_value(true)
                    .multiple_values(true),
            )
            .arg(
                Arg::new("api_key")
                    .long("api-key")
                    .alias("apikey")
                    .help("Your OMDb API key")
                    .long_help("Your OMDb API key (overrides saved value if present)")
                    .takes_value(true),
            )
            .after_long_help("ENVIRONMENT VARIABLES:\n    \
            IMDB_ID_MAX_REQUESTS_PER_SEARCH\n            \
            Adjusts the limit on the number \
            of requests per search. Default is 10\
            ")
    }

    fn process_matches(clap_matches: &ArgMatches) -> Result<Self> {
        let search_term = match clap_matches.values_of("search_term") {
            Some(mut vs) => {
                let mut s = vs.join(" ");
                s.trim_in_place();
                s
            },
            None => {
                if cfg!(not(test)) {
                    user_input::get_search_term()?
                } else {
                    String::new()
                }
            }
        };

        let format = match clap_matches.value_of("format") {
            Some(s) => OutputFormat::try_from(s)?,
            None => RuntimeConfig::default().format,
        };

        let mut interactive = !clap_matches.is_present("non-interactive");
        // atty checks are disabled for testing
        if cfg!(not(test)) {
            use atty::Stream;
            interactive &= atty::is(Stream::Stdout);
            interactive &= atty::is(Stream::Stdin);
        }

        let number_of_results = if interactive || format != Human {
            match clap_matches.value_of("number_of_results") {
                Some(n) => n.parse().unwrap(),
                None => RuntimeConfig::default().number_of_results,
            }
        } else {
            1
        };

        let api_key = clap_matches.value_of("api_key").map(|s| s.to_owned());

        Ok(RuntimeConfig {
            search_term,
            interactive,
            number_of_results,
            filters: Filters::new(clap_matches)?,
            format,
            api_key,
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
            api_key: None,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Human,
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
}

impl TryFrom<&str> for OutputFormat {
    type Error = ClapError;

    fn try_from(s: &str) -> Result<Self, ClapError> {
        let variant = match s.to_ascii_lowercase().as_str() {
            "human" | "plain" => Human,
            "json" => Json,
            #[cfg(feature = "yaml")]
            "yaml" => Yaml,
            _ => return Err(ClapError::InvalidFormat),
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
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-h"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::DisplayHelp);

        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--help"])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::DisplayHelp);
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
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "-r",
                "3",
                "foo",
            ])
            .unwrap();
        assert_eq!(m.value_of("number_of_results"), Some("3"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.number_of_results, 3);
    }

    #[test]
    fn results_long() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--results",
                "7",
                "foo",
            ])
            .unwrap();
        assert_eq!(m.value_of("number_of_results"), Some("7"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.number_of_results, 7);
    }

    #[test]
    fn results_invalid() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--results",
                "bar",
                "foo",
            ])
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
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--non-interactive",
                "foo",
            ])
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
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--non-interactive",
            ])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::MissingRequiredArgument)
    }

    #[test]
    fn multiple_word_search_term() {
        let clap = RuntimeConfig::create_clap_app();
        let matches = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "foo", "bar"])
            .unwrap();
        let search_term_word_count =
            matches.values_of("search_term").unwrap().count();
        assert_eq!(search_term_word_count, 2);

        let config = RuntimeConfig::process_matches(&matches).unwrap();
        assert_eq!(&config.search_term, "foo bar");
    }

    #[test]
    fn format_short() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-f", "json"])
            .unwrap();
        assert_eq!(m.value_of("format"), Some("json"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.format, OutputFormat::Json);

        #[cfg(feature = "yaml")]
        {
            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-f",
                    "yaml",
                ])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("yaml"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }
    }

    #[test]
    fn format_long() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--format",
                "json",
            ])
            .unwrap();
        assert_eq!(m.value_of("format"), Some("json"));

        let config = RuntimeConfig::process_matches(&m).unwrap();
        assert_eq!(config.format, OutputFormat::Json);

        #[cfg(feature = "yaml")]
        {
            let clap = RuntimeConfig::create_clap_app();
            let m = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "--format",
                    "yaml",
                ])
                .unwrap();
            assert_eq!(m.value_of("format"), Some("yaml"));

            let config = RuntimeConfig::process_matches(&m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }
    }

    #[test]
    fn invalid_format() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--format",
                "foo",
            ])
            .unwrap_err();
        assert_eq!(err.kind, clap::ErrorKind::ValueValidation);
    }

    #[test]
    fn api_key() {
        let clap = RuntimeConfig::create_clap_app();
        let m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--api-key",
                "123483",
            ])
            .unwrap();
        assert_eq!(m.value_of("api_key"), Some("123483"));
    }
}
