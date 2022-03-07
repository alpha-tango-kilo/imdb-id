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
pub use user_input::cli::get_api_key;

use crate::omdb::SearchResult;
use omdb::RequestBundle;
use std::cmp::min;
use std::process;
use OutputFormat::*;

fn main() {
    if let Err(why) = app() {
        if why.is_fatal() {
            eprintln!("Error: {why}");
            process::exit(why.error_code());
        }
    }
}

fn app() -> Result<(), FinalError> {
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
            Err(e) => {
                // Suppress not found warnings
                if !matches!(e, DiskError::NotFound(_)) {
                    e.emit_unconditional();
                }
                OnDiskConfig::new_from_prompt()?
            }
        },
    };

    let search_bundle = RequestBundle::new(
        &disk_config.api_key,
        &runtime_config.search_term,
        &runtime_config.filters,
    );
    let allow_reading_time = matches!(runtime_config.format, Human);
    let search_results = search_bundle.get_results(allow_reading_time)?;

    match runtime_config.format {
        Human => {
            if search_results.is_empty() {
                // This isn't run otherwise due to the immediate return
                disk_config.save().emit_unconditional();
                return Err(FinalError::Interaction(
                    InteractivityError::Cancel,
                ));
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
                let selected = user_input::tui(
                    &disk_config.api_key,
                    &search_results[..end_index],
                )?
                .ok_or(FinalError::Interaction(InteractivityError::Cancel))?;
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

    disk_config.save().emit_unconditional();
    Ok(())
}
