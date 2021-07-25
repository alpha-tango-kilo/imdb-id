use crate::{Result, RunError};
#[cfg(not(test))]
use atty::Stream;
use clap::{App, AppSettings, Arg, ArgMatches};
use requestty::Question;

pub struct RuntimeConfig {
    pub search_term: String,
    pub interactive: bool,
    pub number_of_results: usize,
}

impl RuntimeConfig {
    pub fn new() -> Result<Self> {
        RuntimeConfig::process_matches(&RuntimeConfig::create_clap_app().get_matches())
    }

    #[inline]
    fn create_clap_app() -> clap::App<'static> {
        App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("alpha-tango-kilo <git@heyatk.com>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .setting(AppSettings::TrailingVarArg)
            .arg(
                Arg::new("non-interactive")
                    .short('n')
                    .long("non-interactive")
                    .about("Disables interactive features (always picks the first result)")
                    .requires("search_term"),
            )
            .arg(
                Arg::new("number_of_results")
                    .short('r')
                    .long("results")
                    .about("The maximum number of results to show from IMDb")
                    .takes_value(true)
                    .conflicts_with("non-interactive")
                    .validator(|s| s.parse::<usize>().map_err(|_| RunError::ClapNotUsize)),
            )
            .arg(
                Arg::new("search_term")
                    .about("The title of the movie/show you're looking for")
                    .takes_value(true)
                    .multiple(true),
            )
    }

    #[inline]
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
            None => RuntimeConfig::prompt_for_search_term()?,
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

        Ok(RuntimeConfig {
            search_term,
            interactive,
            number_of_results,
        })
    }

    #[inline]
    fn prompt_for_search_term() -> Result<String> {
        let question = Question::input("search_term")
            .message("Please enter the name of the movie/show you're looking for")
            .build();
        Ok(requestty::prompt_one(question)?
            .as_string()
            .unwrap()
            .trim()
            .into())
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            search_term: String::new(),
            interactive: true,
            number_of_results: 10,
        }
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
            .try_get_matches_from(vec![
                env!("CARGO_PKG_NAME"),
                "foo",
                "bar",
            ])
            .unwrap();
        let values = matches.values_of("search_term").unwrap();
        assert_eq!(values.len(), 2);

        let config = RuntimeConfig::process_matches(&matches).unwrap();
        assert_eq!(&config.search_term, "foo bar");
    }
}
