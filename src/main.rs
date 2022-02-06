mod clap_wrap;
mod errors;
mod filters;
pub mod omdb;
mod persistent;
mod user_input;

pub use clap_wrap::*;
pub use errors::*;
pub use filters::*;
pub use persistent::*;
pub use user_input::{choose_result_from, get_api_key};

use omdb::RequestBundle;
use std::cmp::min;
use std::{io, process};
use OutputFormat::*;

fn main() {
    if let Err(why) = app() {
        eprintln!("Error: {why}");
        process::exit(why.error_code());
    }
}

fn app() -> Result<()> {
    let runtime_config = RuntimeConfig::new()?;
    // If an API key is given using the --api-key arg, prefer this over stored
    // value
    let disk_config = match runtime_config.api_key {
        Some(ref api_key) => {
            let mut config = OnDiskConfig {
                api_key: api_key.clone(),
            };
            config.validate()?;
            config
        }
        None => match OnDiskConfig::load() {
            Ok(mut config) => {
                config.validate()?;
                config
            }
            Err(e) => match e.kind() {
                io::ErrorKind::NotFound => OnDiskConfig::new_from_prompt()?,
                _ => return Err(e.into()),
            },
        },
    };

    let search_bundle = RequestBundle::new(
        &disk_config.api_key,
        &runtime_config.search_term,
        &runtime_config.filters,
    );
    let search_results = search_bundle.get_results();

    match runtime_config.format {
        Human => {
            if search_results.is_empty() {
                // This isn't run otherwise due to the immediate return
                disk_config.save()?;
                return Err(RunError::NoSearchResults);
            } else if !runtime_config.interactive || search_results.len() == 1 {
                let search_result = &search_results[0];
                if runtime_config.interactive {
                    eprintln!("Only one result; {search_result}");
                }
                println!("{}", search_result.imdb_id);
            } else {
                // Guaranteed to be interactive
                let end_index =
                    min(search_results.len(), runtime_config.number_of_results);
                let selected =
                    choose_result_from(&search_results[..end_index])?;
                println!("{}", selected.imdb_id);
            }
        }
        Json => {
            let end_index =
                min(runtime_config.number_of_results, search_results.len());
            let json =
                serde_json::to_string_pretty(&search_results[..end_index])?;
            println!("{json}");
        }
        #[cfg(feature = "yaml")]
        Yaml => {
            let end_index =
                min(runtime_config.number_of_results, search_results.len());
            let yaml = serde_yaml::to_string(&search_results[..end_index])?;
            println!("{yaml}");
        }
    }

    disk_config.save()?;

    Ok(())
}
