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

use clap_wrap::OutputFormat::*;
use omdb::{test_api_key, RequestBundle, SearchResult};
use std::borrow::Cow;
use std::cmp::min;
use std::process;
use user_input::cli::get_api_key;

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
    let disk_config = match OnDiskConfig::load() {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            // Suppress not found errors
            if !matches!(e, DiskError::NotFound(_)) {
                e.emit_unconditional();
            }
            None
        }
    };

    // Get API key into one place, regardless as to where it's provided
    let api_key: Option<Cow<str>> = match (runtime_config.api_key, &disk_config)
    {
        // Prefer CLI arg
        (Some(ref s), _) => Some(Cow::Owned(s.clone())),
        (None, Some(OnDiskConfig { api_key })) => Some(Cow::Borrowed(api_key)),
        (None, None) => None,
    };

    // Check/Get API key
    let api_key = match api_key {
        Some(api_key) => match test_api_key(&api_key) {
            Ok(()) => api_key,
            Err(e) => {
                e.emit_non_fatal()?;
                get_api_key()?.into()
            }
        },
        None => get_api_key()?.into(),
    };
    // API key should now always be a good one

    // Update/Save API key to disk if needed
    match disk_config {
        Some(ref cfg) if cfg.api_key != api_key => {
            let new_config = OnDiskConfig {
                api_key: Cow::Borrowed(&api_key),
            };
            new_config.save().emit_unconditional();
        }
        None => {
            let new_config = OnDiskConfig {
                api_key: Cow::Borrowed(&api_key),
            };
            new_config.save().emit_unconditional();
        }
        // API key is same on disk as is being used
        _ => {}
    }

    // Okay let's actually do the search
    let search_bundle = RequestBundle::new(
        &api_key,
        &runtime_config.search_term,
        &runtime_config.filters,
    );
    let allow_reading_time = matches!(runtime_config.format, Human);
    let search_results = search_bundle.get_results(allow_reading_time)?;

    match runtime_config.format {
        Human => {
            if search_results.is_empty() {
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
                let selected =
                    user_input::tui(&api_key, &search_results[..end_index])?
                        .ok_or(FinalError::Interaction(
                            InteractivityError::Cancel,
                        ))?;
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
    Ok(())
}
