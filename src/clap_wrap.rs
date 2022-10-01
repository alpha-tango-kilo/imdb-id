use std::{
    fmt::Write,
    io::{
        stdin,
        stdout,
    },
    ops::BitOr,
    str::FromStr,
};

use clap::{
    builder::NonEmptyStringValueParser,
    Arg,
    ArgAction,
    ArgMatches,
    Command,
};
use trim_in_place::TrimInPlace;

use crate::{
    omdb::MediaType,
    user_input,
    ArgsError,
    Filters,
    OutputFormatParseError,
    Year,
};

#[derive(Debug)]
pub struct RuntimeConfig {
    pub search_term: String,
    pub interactive: bool,
    pub number_of_results: usize,
    pub filters: Filters,
    pub format: OutputFormat,
    pub api_key: Option<String>,
}

impl RuntimeConfig {
    pub fn new() -> Result<Self, ArgsError> {
        RuntimeConfig::process_matches(
            &mut RuntimeConfig::create_clap_app().get_matches(),
        )
    }

    fn create_clap_app() -> Command {
        // Note: any validation will be done in RuntimeConfig::process_matches
        Command::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("alpha-tango-kilo <git@heyatk.com>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(
                Arg::new("non-interactive")
                    .short('n')
                    .long("non-interactive")
                    .help(
                        "Disables interactive features (always picks the \
                         first result)",
                    )
                    .requires("search_term")
                    .action(ArgAction::SetTrue),
            )
            .arg(
                Arg::new("number_of_results")
                    .short('r')
                    .long("results")
                    .help("The maximum number of results to show from IMDb")
                    .num_args(1)
                    .conflicts_with("non-interactive")
                    .value_parser(clap::value_parser!(usize)),
            )
            .arg(
                Arg::new("filter_type")
                    .short('t')
                    .long("type")
                    .help(
                        "Filters results to a specific media type (movie or \
                         series)",
                    )
                    .long_help(
                        "Filters results to a specific media type (movie or \
                         series). Can be given multiple times",
                    )
                    .num_args(1)
                    .action(ArgAction::Append)
                    .value_parser(MediaType::from_str),
            )
            .arg(
                Arg::new("filter_year")
                    .short('y')
                    .long("year")
                    .help("Filter results to a specific year")
                    .long_help(
                        "Filters results to a specific year, or range of \
                         years\nMedia which has no year specified will always \
                         be included\nRanges are fully inclusive\nExamples: \
                         2021, 1990-2000, 2000- (2000 onwards), -2000 (before \
                         2000)",
                    )
                    .num_args(1)
                    .allow_hyphen_values(true)
                    .value_parser(Year::from_str),
            )
            .arg(
                Arg::new("format")
                    .short('f')
                    .long("format")
                    .help("Change output format to desired standard")
                    .long_help(
                        "Change output format to desired standard\nFormats \
                         are only available if you opted-IN at \
                         installation\nAll the formats imdb-id can support \
                         are: json, yaml",
                    )
                    .num_args(1)
                    .value_parser(OutputFormat::from_str),
            )
            .arg(
                Arg::new("search_term")
                    .help("The title of the movie/show you're looking for")
                    .trailing_var_arg(true)
                    .num_args(0..),
            )
            .arg(
                Arg::new("api_key")
                    .long("api-key")
                    .alias("apikey")
                    .help("Your OMDb API key")
                    .long_help(
                        "Your OMDb API key (overrides saved value if present)",
                    )
                    .num_args(1)
                    .value_parser(NonEmptyStringValueParser::new()),
            )
            .after_long_help(
                "ENVIRONMENT VARIABLES:\n    \
                 IMDB_ID_MAX_REQUESTS_PER_SEARCH\n            Adjusts the \
                 limit on the number of requests per search. Default is 10",
            )
    }

    fn process_matches(
        clap_matches: &mut ArgMatches,
    ) -> Result<Self, ArgsError> {
        let format = clap_matches
            .remove_one::<OutputFormat>("format")
            .unwrap_or_default();

        let mut interactive = !clap_matches.get_flag("non-interactive");
        // TTY checks are disabled for testing
        if cfg!(not(test)) {
            use crossterm::tty::IsTty;
            interactive &= stdout().is_tty();
            interactive &= stdin().is_tty();
        }

        let number_of_results =
            if interactive || !matches!(format, OutputFormat::Human) {
                clap_matches
                    .remove_one::<usize>("number_of_results")
                    .unwrap_or(RuntimeConfig::default().number_of_results)
            } else {
                1
            };

        let api_key = clap_matches.remove_one::<String>("api_key");

        let types = clap_matches
            .remove_many::<MediaType>("filter_type")
            .map(|mts| mts.reduce(BitOr::bitor).unwrap())
            .unwrap_or(MediaType::ALL);

        // Match used so ? can be used
        let years = clap_matches.remove_one::<Year>("filter_year");

        let filters = Filters { types, years };

        let search_term =
            match clap_matches.remove_many::<String>("search_term") {
                Some(mut words) => {
                    let mut search_term = words.next().unwrap();
                    search_term.trim_in_place();
                    words.for_each(|word| {
                        write!(search_term, " {} ", word.trim()).unwrap();
                    });
                    // Remove trailing extra space
                    search_term.pop();
                    search_term
                },
                None => {
                    if cfg!(not(test)) {
                        user_input::cli::get_search_term(filters.types)?
                    } else {
                        String::new()
                    }
                },
            };

        Ok(RuntimeConfig {
            search_term,
            interactive,
            number_of_results,
            filters,
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
            format: OutputFormat::default(),
            api_key: None,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum OutputFormat {
    Human,
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Human
    }
}

impl FromStr for OutputFormat {
    type Err = OutputFormatParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use OutputFormat::*;
        use OutputFormatParseError::*;
        match s.to_ascii_lowercase().as_str() {
            "human" | "plain" => Ok(Human),
            "json" => Ok(Json),
            #[cfg(feature = "yaml")]
            "yaml" => Ok(Yaml),
            #[cfg(not(feature = "yaml"))]
            not_installed @ "yaml" => {
                Err(NotInstalled(not_installed.to_owned()))
            },
            other => Err(Unrecognised(other.to_owned())),
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use clap::error::ErrorKind;

    use super::*;

    #[test]
    fn clap() {
        RuntimeConfig::create_clap_app().debug_assert();
    }

    #[test]
    fn help() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-h"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);

        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--help"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    }

    #[test]
    fn version() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-V"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);

        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "--version"])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    }

    #[test]
    fn results_short() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "-r",
                "3",
                "foo",
            ])
            .unwrap();
        assert_eq!(m.get_one::<usize>("number_of_results"), Some(&3));

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
        assert_eq!(config.number_of_results, 3);
    }

    #[test]
    fn results_long() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--results",
                "7",
                "foo",
            ])
            .unwrap();
        assert_eq!(m.get_one::<usize>("number_of_results"), Some(&7));

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
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
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn non_interactive_short() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-n", "foo"])
            .unwrap();
        assert!(m.get_flag("non-interactive"));

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
        assert!(!config.interactive);
        assert_eq!(config.number_of_results, 1);
    }

    #[test]
    fn non_interactive_long() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--non-interactive",
                "foo",
            ])
            .unwrap();
        assert!(m.get_flag("non-interactive"));

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
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
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
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
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument)
    }

    #[test]
    fn multiple_word_search_term() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "foo", "bar"])
            .unwrap();
        let search_term_word_count =
            m.get_many::<String>("search_term").unwrap().count();
        assert_eq!(search_term_word_count, 2);

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
        assert_eq!(&config.search_term, "foo bar");
    }

    #[test]
    fn format_short() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![env!("CARGO_PKG_NAME"), "-f", "json"])
            .unwrap();
        assert_eq!(
            m.get_one::<OutputFormat>("format"),
            Some(&OutputFormat::Json)
        );

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
        assert_eq!(config.format, OutputFormat::Json);

        #[cfg(feature = "yaml")]
        {
            let clap = RuntimeConfig::create_clap_app();
            let mut m = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-f",
                    "yaml",
                ])
                .unwrap();
            assert_eq!(
                m.get_one::<OutputFormat>("format"),
                Some(&OutputFormat::Yaml)
            );

            let config = RuntimeConfig::process_matches(&mut m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }
    }

    #[test]
    fn format_long() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--format",
                "json",
            ])
            .unwrap();
        assert_eq!(
            m.get_one::<OutputFormat>("format"),
            Some(&OutputFormat::Json)
        );

        let config = RuntimeConfig::process_matches(&mut m).unwrap();
        assert_eq!(config.format, OutputFormat::Json);

        #[cfg(feature = "yaml")]
        {
            let clap = RuntimeConfig::create_clap_app();
            let mut m = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "--format",
                    "yaml",
                ])
                .unwrap();
            assert_eq!(
                m.get_one::<OutputFormat>("format"),
                Some(&OutputFormat::Yaml)
            );

            let config = RuntimeConfig::process_matches(&mut m).unwrap();
            assert_eq!(config.format, OutputFormat::Yaml);
        }
    }

    #[cfg(not(feature = "yaml"))]
    #[test]
    fn not_installed_format() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--format",
                "yaml",
            ])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn unrecognised_format() {
        let clap = RuntimeConfig::create_clap_app();
        let err = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--format",
                "foo",
            ])
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn api_key() {
        let clap = RuntimeConfig::create_clap_app();
        let mut m = clap
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "--api-key",
                "123483",
            ])
            .unwrap();
        assert_eq!(
            m.remove_one::<String>("api_key").as_deref(),
            Some("123483")
        );
    }

    mod filters {
        use clap::ArgMatches;

        use crate::{
            filters::CURRENT_YEAR,
            omdb::MediaType,
            Filters,
            RuntimeConfig,
            Year,
        };

        fn from_matches(clap_matches: &mut ArgMatches) -> Filters {
            RuntimeConfig::process_matches(clap_matches)
                .unwrap()
                .filters
        }

        #[test]
        fn media_type() {
            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-t",
                    "series",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                types: MediaType::SERIES,
                years: None,
            });

            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-t",
                    "Movie",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                types: MediaType::MOVIE,
                ..Default::default()
            });
        }

        #[test]
        fn year() {
            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                years: Some(Year(1980..=1980)),
                ..Default::default()
            });

            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-2010",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                years: Some(Year(1980..=2010)),
                ..Default::default()
            });

            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                years: Some(Year(1980..=*CURRENT_YEAR)),
                ..Default::default()
            });

            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "-2010",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                years: Some(Year(0..=2010)),
                ..Default::default()
            });
        }

        #[test]
        fn year_inverted() {
            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "2010-1980",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                years: Some(Year(1980..=2010)),
                ..Default::default()
            });
        }

        #[test]
        fn mixed() {
            let clap = RuntimeConfig::create_clap_app();
            let mut clap_matches = clap
                .try_get_matches_from(vec![
                    env!("CARGO_PKG_NAME"),
                    "-y",
                    "1980-2010",
                    "-t",
                    "Movies",
                ])
                .unwrap();
            let filters = from_matches(&mut clap_matches);
            assert_eq!(filters, Filters {
                types: MediaType::MOVIE,
                years: Some(Year(1980..=2010)),
            });
        }
    }
}
