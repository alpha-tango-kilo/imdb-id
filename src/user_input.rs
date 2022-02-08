pub use self::tui::tui;
use crate::InteractivityError;

type Result<T, E = InteractivityError> = std::result::Result<T, E>;

pub mod cli {
    use super::{InteractivityError, Result};
    use crate::omdb::test_api_key;
    use crate::SignUpError;
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Confirm, Input, Select};
    use lazy_regex::{lazy_regex, Lazy, Regex};
    use lazy_static::lazy_static;
    use minreq::get;
    use std::fmt::Display;
    use std::ops::Deref;

    const SIGN_UP_URL: &str = "https://www.omdbapi.com/apikey.aspx";
    const AUTOMATED_SIGN_UP_URL: &str = "https://www.omdbapi.com/apikey.aspx?__EVENTTARGET=&__EVENTARGUMENT=&__LASTFOCUS=&__VIEWSTATE=%2FwEPDwUKLTIwNDY4MTIzNQ9kFgYCAQ9kFggCAQ8QDxYCHgdDaGVja2VkaGRkZGQCAw8QDxYCHwBnZGRkZAIFDxYCHgdWaXNpYmxlaGQCBw8WAh8BZ2QCAg8WAh8BaGQCAw8WAh8BaGQYAQUeX19Db250cm9sc1JlcXVpcmVQb3N0QmFja0tleV9fFgMFC3BhdHJlb25BY2N0BQtwYXRyZW9uQWNjdAUIZnJlZUFjY3SZmkfBgEVOtEhBRPgn0xJZZDjfMEiMoho3O8lIVPYLXg%3D%3D&__VIEWSTATEGENERATOR=5E550F58&__EVENTVALIDATION=%2FwEdAAhq8u7G6E8iNQTDLBqGZykXmSzhXfnlWWVdWIamVouVTzfZJuQDpLVS6HZFWq5fYphdL1XrNEjnC%2FKjNya%2Bmqh8hRPnM5dWgso2y7bj7kVNLSFbtYIt24Lw6ktxrd5Z67%2F4LFSTzFfbXTFN5VgQX9Nbzfg78Z8BXhXifTCAVkevd2U20ItIGqFIf8giu%2B0PAasvwu4KgXUo9rywyT%2ByOXGt&at=freeAcct&Button1=Submit";
    const SUCCESSFUL_SIGN_UP_NEEDLE: &str =
        "A verification link to activate your key was sent to: ";

    // https://www.emailregex.com/
    static EMAIL_REGEX: Lazy<Regex> = lazy_regex!(
        r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#
    );

    lazy_static! {
        static ref THEME: ColorfulTheme = Default::default();
    }

    pub fn get_api_key() -> Result<String> {
        let has_key = Confirm::with_theme(THEME.deref())
            .with_prompt("Do you have an OMDb API key?")
            .default(false)
            .interact()?;

        if !has_key {
            if let Err(why) = omdb_sign_up() {
                match opener::open_browser(SIGN_UP_URL) {
                    Ok(()) => eprintln!("Automated sign up failed (sorry!), website opened ({why})"),
                    Err(_) => eprintln!("Automated sign up failed (sorry!), please visit {SIGN_UP_URL} ({why})"),
                }
            }
        }

        let api_key = Input::with_theme(THEME.deref())
            .with_prompt("Please enter your API key")
            .validate_with(|api_key: &String| test_api_key(api_key))
            .interact_text()?;

        Ok(api_key)
    }

    fn omdb_sign_up() -> Result<(), SignUpError> {
        let email = Input::<String>::with_theme(THEME.deref())
            .with_prompt(
                "Please enter an email to receive your OMDb API key to",
            )
            .validate_with(|email: &String| {
                match EMAIL_REGEX.is_match(&email.to_lowercase()) {
                    true => Ok(()),
                    false => Err("Email appears to be invalid"),
                }
            })
            .interact_text()
            .map_err(InteractivityError::from)?
            .to_lowercase();
        let first_name = Input::<String>::with_theme(THEME.deref())
            .with_prompt("Please input your first name (OMDb requests this)")
            .default(String::from("Joe"))
            .interact_text()
            .map_err(InteractivityError::from)?;
        let last_name = Input::<String>::with_theme(THEME.deref())
            .with_prompt("Please input your last name (OMDb requests this)")
            .default(String::from("Bloggs"))
            .interact_text()
            .map_err(InteractivityError::from)?;
        let r#use = "Searching the API with imdb-id (https://codeberg.org/alpha-tango-kilo/imdb-id)";

        let request = get(format!(
            "{AUTOMATED_SIGN_UP_URL}&Email2={email}&FirstName={first_name}&LastName={last_name}&TextArea1={use}",
            email = urlencoding::encode(&email),
            first_name = urlencoding::encode(&first_name),
            last_name = urlencoding::encode(&last_name),
            r#use = urlencoding::encode(r#use),
        ));
        let response = request.send()?;
        let body = response.as_str()?;

        match body.contains(SUCCESSFUL_SIGN_UP_NEEDLE) {
            true => {
                println!("Sign up was successful, check your email");
                Ok(())
            }
            false => Err(SignUpError::NeedleNotFound),
        }
    }

    pub fn get_search_term() -> Result<String> {
        let question = Input::with_theme(THEME.deref())
            .with_prompt(
                "Please enter the name of the movie/show you're looking for",
            )
            .interact_text()?;
        Ok(question)
    }

    pub fn choose_result_from<E: Display>(entries: &[E]) -> Result<&E> {
        Select::with_theme(THEME.deref())
            .with_prompt("Pick the correct search result (Esc or Q to quit)")
            .items(entries)
            .interact_opt()?
            .map(|index| &entries[index])
            .ok_or(InteractivityError::Cancel)
    }
}

mod tui {
    use super::InteractivityError;
    use crate::SearchResult;
    use crossterm::event::{
        DisableMouseCapture, EnableMouseCapture, Event, KeyCode,
    };
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    };
    use crossterm::{event, execute};
    use std::io;
    use std::time::{Duration, Instant};
    use tui::backend::CrosstermBackend;
    use tui::layout::{Constraint, Direction, Layout};
    use tui::widgets::{Block, Borders, List, ListItem, ListState};
    use tui::Terminal;

    const TICK_RATE: Duration = Duration::from_millis(250);

    // Based on https://github.com/fdehau/tui-rs/blob/85939306e3fef04322483954d3031399a58d002b/examples/list.rs#L20-L64
    struct StatefulList<'a, T> {
        state: ListState,
        underlying: &'a [T],
        list_items: Vec<ListItem<'a>>,
    }

    impl<'a, T> StatefulList<'a, T>
    where
        &'a T: Into<ListItem<'a>>,
    {
        fn new(items: &'a [T]) -> Self {
            let list_items = items.iter().map(Into::into).collect();

            StatefulList {
                state: ListState::default(),
                underlying: items,
                list_items,
            }
        }

        fn next(&mut self) {
            let index = match self.state.selected() {
                Some(index) => (index + 1) % self.underlying.len(),
                None => 0,
            };
            self.state.select(Some(index));
        }

        fn previous(&mut self) {
            let index = match self.state.selected() {
                Some(index) => {
                    index.checked_sub(1).unwrap_or(self.underlying.len() - 1)
                }
                None => 0,
            };
            self.state.select(Some(index));
        }

        fn items(&self) -> Vec<ListItem<'a>> {
            self.list_items.clone()
        }

        fn current(&self) -> &'a T {
            let index = self.state.selected().unwrap_or_default();
            &self.underlying[index]
        }
    }

    pub fn tui(
        entries: &[SearchResult],
    ) -> Result<&SearchResult, InteractivityError> {
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
                        [
                            Constraint::Percentage(40),
                            Constraint::Percentage(60),
                        ]
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

            if event::poll(timeout)? {
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

    impl<'a> From<&SearchResult> for ListItem<'a> {
        fn from(sr: &SearchResult) -> Self {
            ListItem::new(sr.to_string())
        }
    }
}
