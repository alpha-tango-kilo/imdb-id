#![cfg_attr(debug_assertions, allow(dead_code, unused_imports))]

use imdb_id::omdb::query_by_title;
use imdb_id::OutputFormat::*;
use imdb_id::*;
use std::cmp::min;
use std::process;

fn main() {
    match query_by_title("***REMOVED***", "kings") {
        Ok(omdb) => println!("{}", omdb),
        Err(why) => eprintln!("{}", why),
    }

    /*if let Err(why) = app() {
        eprintln!("Error: {}", why);
        process::exit(why.error_code());
    }*/
}

fn app() -> Result<()> {
    let config = RuntimeConfig::new()?;

    let fragments = request_and_scrape(&config.search_term)?;
    let search_results = SearchResult::try_many_lossy(fragments, &config.filters);

    match config.format {
        Human => {
            if search_results.len() == 0 {
                return Err(RunError::NoSearchResults);
            } else if !config.interactive || search_results.len() == 1 {
                let search_result = search_results.get(0).unwrap();
                if config.interactive {
                    eprintln!("Only one result; {}", search_result);
                }
                println!("{}", search_result.id);
            } else {
                // Guaranteed to be interactive
                let mut pager = Pager::new(&search_results, &config);
                let selected = pager.ask()?;
                println!("{}", selected.id);
            }
        }
        Json => {
            let end_index = min(config.number_of_results, search_results.len());
            let json = serde_json::to_string_pretty(&search_results[..end_index])?;
            println!("{}", json);
        }
        #[cfg(feature = "yaml")]
        Yaml => {
            let end_index = min(config.number_of_results, search_results.len());
            let yaml = serde_yaml::to_string(&search_results[..end_index])?;
            println!("{}", yaml);
        }
    }

    Ok(())
}
