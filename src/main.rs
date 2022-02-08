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

use crate::omdb::SearchResult;
use crate::user_input::StatefulList;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{event, execute};
use omdb::RequestBundle;
use std::cmp::min;
use std::time::{Duration, Instant};
use std::{io, process};
use tui::backend::CrosstermBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::widgets::{Block, Borders, List};
use tui::Terminal;
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
                // DiskError on read is never fatal, so unwrap is fine
                e.emit_unconditional();
                OnDiskConfig::new_from_prompt()?
            }
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
                disk_config.save().emit_unconditional();
                return Err(FinalError::NoSearchResults);
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
                let selected = tui(&search_results[..end_index])?;
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

const TICK_RATE: Duration = Duration::from_millis(250);

fn tui(entries: &[SearchResult]) -> Result<&SearchResult, InteractivityError> {
    let mut list = StatefulList::new(entries);

    let mut stdout = io::stdout();

    // Crossterm setup
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);

    // TUI
    let mut terminal = Terminal::new(backend)?;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [Constraint::Percentage(40), Constraint::Percentage(60)]
                        .as_slice(),
                )
                .split(f.size());

            let results = List::new(list.items())
                .block(
                    Block::default()
                        .title("Search results")
                        .borders(Borders::ALL),
                )
                .highlight_symbol(">> ");
            f.render_stateful_widget(results, chunks[0], &mut list.state);

            let info =
                Block::default().title("Information").borders(Borders::ALL);
            f.render_widget(info, chunks[1]);
        })?;

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => break,
                    KeyCode::Up | KeyCode::Char('k') => list.previous(),
                    KeyCode::Down | KeyCode::Char('j') => list.next(),
                    _ => {}
                }
            }
        }
        last_tick = Instant::now();
    }

    // Crossterm unwind
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    Ok(list.current())
}
