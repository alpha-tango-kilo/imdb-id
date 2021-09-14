use imdb_id::OutputFormat::*;
use imdb_id::*;
use std::cmp::min;
use std::process;

fn main() {
    if let Err(why) = app() {
        eprintln!("Error: {}", why);
        process::exit(why.error_code());
    }
}

fn app() -> Result<()> {
    let config = RuntimeConfig::new()?;
    let client = reqwest::Client::new();
    let api_key = match std::env::var("OMDB_APIKEY") {
        Ok(key) => key,
        Err(_) => get_api_key(&client)?,
    };

    let search_results = omdb::search_by_title(&api_key, &client, &config.search_term)?;

    match config.format {
        Human => {
            if search_results.entries.len() == 0 {
                return Err(RunError::NoSearchResults);
            } else if !config.interactive || search_results.entries.len() == 1 {
                let search_result = search_results.entries.get(0).unwrap();
                if config.interactive {
                    eprintln!("Only one result; {}", search_result);
                }
                println!("{}", search_result.imdb_id);
            } else {
                // Guaranteed to be interactive
                let mut pager = Pager::new(&search_results.entries, &config);
                let selected = pager.ask()?;
                println!("{}", selected.imdb_id);
            }
        }
        Json => {
            let end_index = min(config.number_of_results, search_results.entries.len());
            let json = serde_json::to_string_pretty(&search_results.entries[..end_index])?;
            println!("{}", json);
        }
        #[cfg(feature = "yaml")]
        Yaml => {
            let end_index = min(config.number_of_results, search_results.entries.len());
            let yaml = serde_yaml::to_string(&search_results.entries[..end_index])?;
            println!("{}", yaml);
        }
    }

    Ok(())
}
