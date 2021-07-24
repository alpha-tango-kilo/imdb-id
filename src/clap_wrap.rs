use atty::Stream;
use clap::{App, Arg};

pub struct RuntimeConfig {
    pub search_term: String,
    pub interactive: bool,
    pub number_of_results: usize,
}

impl RuntimeConfig {
    pub fn new() -> Self {
        let clap = App::new(env!("CARGO_PKG_NAME"))
            .version(env!("CARGO_PKG_VERSION"))
            .author("alpha-tango-kilo <git@heyatk.com>")
            .about(env!("CARGO_PKG_DESCRIPTION"))
            .arg(Arg::new("non-interactive")
                .short('n')
                .long("non-interactive")
                .about("Disables interactive features (effectively makes search \"I'm feeling lucky\")"))
            .arg(Arg::new("number_of_results")
                .short('r')
                .long("results")
                .about("The maximum number of results to show from IMDb")
                .takes_value(true)
                // TODO: consider wrapping in RunError
                .validator(|s| s.parse::<usize>()))
            .arg(Arg::new("search_term")
                .about("The title of the movie/show you're looking for")
                .required(true))
            .get_matches();

        let search_term = clap.value_of("search_term").unwrap().to_string();

        let interactive = !clap.is_present("non-interactive")
            && atty::is(Stream::Stdout)
            && atty::is(Stream::Stdin);

        let number_of_results = if interactive {
            match clap.value_of("number_of_results") {
                Some(n) => n.parse().unwrap(),
                None => RuntimeConfig::default().number_of_results,
            }
        } else {
            1
        };

        RuntimeConfig {
            search_term,
            interactive,
            number_of_results,
        }
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
