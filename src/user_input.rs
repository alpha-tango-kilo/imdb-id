pub use self::tui::tui;
use crate::InteractivityError;

type Result<T, E = InteractivityError> = std::result::Result<T, E>;

pub mod cli {
    use super::{InteractivityError, Result};
    use crate::omdb::{test_api_key, MediaType};
    use crate::{FinalError, MaybeFatal, SignUpError};
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Confirm, Input};
    use lazy_regex::{lazy_regex, Regex};
    use minreq::get;
    use once_cell::sync::Lazy;
    use std::ops::Deref;

    const SIGN_UP_URL: &str = "https://www.omdbapi.com/apikey.aspx";
    const AUTOMATED_SIGN_UP_URL: &str = "https://www.omdbapi.com/apikey.aspx?__EVENTTARGET=&__EVENTARGUMENT=&__LASTFOCUS=&__VIEWSTATE=%2FwEPDwUKLTIwNDY4MTIzNQ9kFgYCAQ9kFggCAQ8QDxYCHgdDaGVja2VkaGRkZGQCAw8QDxYCHwBnZGRkZAIFDxYCHgdWaXNpYmxlaGQCBw8WAh8BZ2QCAg8WAh8BaGQCAw8WAh8BaGQYAQUeX19Db250cm9sc1JlcXVpcmVQb3N0QmFja0tleV9fFgMFC3BhdHJlb25BY2N0BQtwYXRyZW9uQWNjdAUIZnJlZUFjY3SZmkfBgEVOtEhBRPgn0xJZZDjfMEiMoho3O8lIVPYLXg%3D%3D&__VIEWSTATEGENERATOR=5E550F58&__EVENTVALIDATION=%2FwEdAAhq8u7G6E8iNQTDLBqGZykXmSzhXfnlWWVdWIamVouVTzfZJuQDpLVS6HZFWq5fYphdL1XrNEjnC%2FKjNya%2Bmqh8hRPnM5dWgso2y7bj7kVNLSFbtYIt24Lw6ktxrd5Z67%2F4LFSTzFfbXTFN5VgQX9Nbzfg78Z8BXhXifTCAVkevd2U20ItIGqFIf8giu%2B0PAasvwu4KgXUo9rywyT%2ByOXGt&at=freeAcct&Button1=Submit";
    const SUCCESSFUL_SIGN_UP_NEEDLE: &str =
        "A verification link to activate your key was sent to: ";

    // https://www.emailregex.com/
    static EMAIL_REGEX: Lazy<Regex> = lazy_regex!(
        r#"(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])"#
    );

    static THEME: Lazy<ColorfulTheme> = Lazy::new(ColorfulTheme::default);

    // Only errors returned are fatal, hence FinalError
    // Will only ever be FinalError::Interactivity or FinalError::ApiKey
    pub fn get_api_key() -> Result<String, FinalError> {
        let has_key = Confirm::with_theme(THEME.deref())
            .with_prompt("Do you have an OMDb API key?")
            .default(false)
            .interact()
            .map_err(InteractivityError::from_cli)?;

        if !has_key {
            use InteractivityError::Cancel;
            match omdb_sign_up() {
                Ok(()) => {}
                // Quit out if we notice the user is trying to cancel
                Err(SignUpError::Interactivity(Cancel)) => {
                    return Err(FinalError::Interaction(Cancel));
                }
                Err(why) => {
                    match opener::open_browser(SIGN_UP_URL) {
                        Ok(()) => eprintln!("Automated sign up failed (sorry!), website opened ({why})"),
                        Err(_) => eprintln!("Automated sign up failed (sorry!), please visit {SIGN_UP_URL} ({why})"),
                    }
                }
            }
        }

        // Don't validate using dialoguer's built-in capabilities, as some
        // errors may be fatal
        loop {
            let api_key = Input::<String>::with_theme(THEME.deref())
                .with_prompt("Please enter your API key")
                .interact_text()
                .map_err(InteractivityError::from_cli)?;
            match test_api_key(&api_key) {
                Ok(()) => return Ok(api_key),
                Err(fatal) if fatal.is_fatal() => return Err(fatal.into()),
                Err(warn) => {
                    eprintln!("Bad API key: {warn}");
                },
            }
        }
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
            .map_err(InteractivityError::from_cli)?
            .to_lowercase();
        let first_name = Input::<String>::with_theme(THEME.deref())
            .with_prompt("Please input your first name (OMDb requests this)")
            .default(String::from("Joe"))
            .interact_text()
            .map_err(InteractivityError::from_cli)?;
        let last_name = Input::<String>::with_theme(THEME.deref())
            .with_prompt("Please input your last name (OMDb requests this)")
            .default(String::from("Bloggs"))
            .interact_text()
            .map_err(InteractivityError::from_cli)?;
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
            },
            false => Err(SignUpError::NeedleNotFound),
        }
    }

    pub fn get_search_term(types: MediaType) -> Result<String> {
        let question = Input::with_theme(THEME.deref())
            .with_prompt(format!(
                "Please enter the name of the {types} you're looking for"
            ))
            .interact_text()
            .map_err(InteractivityError::from_cli)?;
        Ok(question)
    }
}

pub mod tui {
    use super::InteractivityError;
    use crate::omdb::{get_entry, Entry};
    use crate::{RequestError, SearchResult};
    use crossterm::event::{Event, KeyCode};
    use crossterm::terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    };
    use crossterm::{event, execute};
    use itertools::Itertools;
    use once_cell::sync::Lazy;
    use ratatui::backend::CrosstermBackend;
    use ratatui::layout::{Constraint, Direction, Layout};
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Wrap,
    };
    use ratatui::Terminal;
    use std::fmt::Display;
    use std::io;
    use std::io::Stdout;

    const HIGHLIGHT_SYMBOL: &str = "> ";
    const MIN_MARGIN: usize = 1;

    static BOLD: Lazy<Style> =
        Lazy::new(|| Style::default().add_modifier(Modifier::BOLD));

    struct ListItemList {
        items: Vec<ListItem<'static>>,
        width: usize,
    }

    impl ListItemList {
        pub fn new<T: Display>(items: &[T], width: usize) -> Self {
            let items = items
                .iter()
                .map(|t| {
                    let mut s = t.to_string();
                    textwrap::fill_inplace(&mut s, width);
                    ListItem::new(s)
                })
                .collect();

            ListItemList { items, width }
        }

        pub fn items_cloned(&self) -> Vec<ListItem<'static>> {
            self.items.clone()
        }
    }

    struct StatefulList<'a> {
        state: ListState,
        underlying: &'a [SearchResult],
        list_items: Option<ListItemList>,
        entry_paragraphs: Vec<Option<Paragraph<'static>>>,
    }

    impl<'a> StatefulList<'a> {
        fn new(items: &'a [SearchResult]) -> Self {
            debug_assert!(
                !items.is_empty(),
                "Can't construct StatefulList without items"
            );
            let mut state = ListState::default();
            state.select(Some(0));

            StatefulList {
                state,
                underlying: items,
                list_items: None,
                entry_paragraphs: vec![None; items.len()],
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
                },
                None => 0,
            };
            self.state.select(Some(index));
        }

        fn items(&mut self, width: usize) -> Vec<ListItem<'static>> {
            match &self.list_items {
                Some(li) if li.width == width => li.items_cloned(),
                _ => {
                    let lil = ListItemList::new(self.underlying, width);
                    let items = lil.items_cloned();
                    self.list_items = Some(lil);
                    items
                },
            }
        }

        fn entry(
            &mut self,
            api_key: &str,
        ) -> Result<Paragraph<'static>, RequestError> {
            let index = self.state.selected().unwrap();
            match &self.entry_paragraphs[index] {
                Some(entry) => Ok(entry.clone()),
                None => {
                    // Make web request for entry
                    let imdb_id = &self.underlying[index].imdb_id;
                    let entry = get_entry(api_key, imdb_id)?;
                    let paragraph = entry_to_paragraph(entry);
                    self.entry_paragraphs[index] = Some(paragraph.clone());
                    Ok(paragraph)
                },
            }
        }

        fn current(&self) -> usize {
            self.state
                .selected()
                .expect("Stateful list should always have a selected item")
        }
    }

    pub enum TuiOutcome<'a> {
        Picked(&'a SearchResult),
        PickedError(&'a SearchResult, RequestError),
        Quit,
    }

    pub fn tui<'a>(
        api_key: &str,
        entries: &'a [SearchResult],
    ) -> Result<TuiOutcome<'a>, InteractivityError> {
        let mut status_list = StatefulList::new(entries);
        let mut current_entry_error = None;

        let mut stdout = io::stdout();

        // Crossterm setup
        enable_raw_mode().map_err(InteractivityError::Crossterm)?;
        execute!(stdout, EnterAlternateScreen)
            .map_err(InteractivityError::Crossterm)?;
        let backend = CrosstermBackend::new(stdout);

        // TUI
        let mut terminal =
            Terminal::new(backend).map_err(InteractivityError::Tui)?;

        // Could gag stdout/stderr with https://lib.rs/crates/gag if this is
        // needed in the future
        loop {
            terminal
                .draw(|f| {
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

                    // subtract width of borders
                    let width = chunks[0].width.saturating_sub(2) as usize;
                    let width = width.saturating_sub(HIGHLIGHT_SYMBOL.len());
                    let width = width.saturating_sub(MIN_MARGIN);
                    let items = status_list.items(width);

                    let selection_list = List::new(items)
                        .block(
                            Block::default()
                                .title("[Search results]")
                                .borders(Borders::ALL),
                        )
                        .highlight_symbol(HIGHLIGHT_SYMBOL);

                    f.render_stateful_widget(
                        selection_list,
                        chunks[0],
                        &mut status_list.state,
                    );

                    match status_list.entry(api_key) {
                        Ok(entry) => {
                            f.render_widget(entry, chunks[1]);
                            current_entry_error = None;
                        },
                        Err(why) => {
                            // Fall back on rendering the error as a Paragraph
                            f.render_widget(
                                error_to_paragraph(&why),
                                chunks[1],
                            );
                            current_entry_error = Some(why);
                        },
                    }
                })
                .map_err(InteractivityError::Tui)?;

            // Blocks until key press or terminal resize
            if let Event::Key(key) =
                event::read().map_err(InteractivityError::Crossterm)?
            {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        unwind(terminal.backend_mut())
                            .map_err(InteractivityError::Crossterm)?;
                        return Ok(TuiOutcome::Quit);
                    },
                    KeyCode::Enter => break,
                    KeyCode::Up | KeyCode::Char('k') => status_list.previous(),
                    KeyCode::Down | KeyCode::Char('j') => status_list.next(),
                    _ => {},
                }
            }
        }

        // Crossterm unwind
        unwind(terminal.backend_mut())
            .map_err(InteractivityError::Crossterm)?;
        let chosen = &entries[status_list.current()];
        match current_entry_error {
            None => Ok(TuiOutcome::Picked(chosen)),
            Some(err) => Ok(TuiOutcome::PickedError(chosen, err)),
        }
    }

    // Crossterm unwind
    fn unwind(stdout: &mut CrosstermBackend<Stdout>) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(stdout, LeaveAlternateScreen)
    }

    fn entry_to_paragraph(entry: Entry) -> Paragraph<'static> {
        let Entry {
            title,
            year,
            rating,
            runtime,
            genres,
            actors,
            plot,
            seasons,
            ..
        } = entry;
        let mut information = Vec::with_capacity(6);
        // Line 1: title & year
        information.push(Line::from(vec![
            Span::styled("Title: ", *BOLD),
            Span::raw(title),
            Span::styled(
                format!(" ({year})"),
                Style::default().add_modifier(Modifier::DIM),
            ),
        ]));
        // Line 2: run time
        match (seasons, runtime) {
            (Some(seasons), Some(runtime)) => {
                // e.g. Seasons: 6 (45 minutes per episode)
                information.push(Line::from(vec![
                    Span::styled("Seasons: ", *BOLD),
                    Span::raw(seasons.to_string()),
                    Span::raw(" ("),
                    Span::raw(runtime),
                    Span::raw(" per episode)"),
                ]));
            },
            (Some(seasons), None) => {
                // e.g. Seasons: 6
                information.push(Line::from(vec![
                    Span::styled("Seasons: ", *BOLD),
                    Span::raw(seasons.to_string()),
                ]));
            },
            (None, Some(runtime)) => {
                // e.g. Run time: 120 minutes
                information.push(Line::from(vec![
                    Span::styled("Run time: ", *BOLD),
                    Span::raw(runtime),
                ]));
            },
            (None, None) => {},
        }
        // Line 3: rating
        if let Some(rating) = rating {
            information.push(Line::from(vec![
                Span::styled("IMDb Rating: ", *BOLD),
                Span::raw(rating.to_string()),
            ]));
        }
        // Line 4: genres
        if let Some(genres) = genres {
            information.push(Line::from(vec![
                Span::styled("Genre(s): ", *BOLD),
                Span::raw(format_list(&genres)),
            ]));
        }
        // Line 5: actors
        if let Some(actors) = actors {
            information.push(Line::from(vec![
                Span::styled("Actor(s): ", *BOLD),
                Span::raw(format_list(&actors)),
            ]));
        }
        // Line 6: plot
        if let Some(plot) = plot {
            information.push(Line::from(vec![
                Span::styled("Plot: ", *BOLD),
                Span::raw(plot),
            ]));
        }

        Paragraph::new(information)
            .block(
                Block::default()
                    .title("[Information]")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false })
    }

    fn error_to_paragraph(error: &RequestError) -> Paragraph<'static> {
        let mut text = vec![
            Line::from(Span::styled("Failed to load entry", *BOLD)),
            Line::from(Span::styled("This error will be printed for easier copying if you choose it", *BOLD)),
        ];

        // Interpret newlines by putting each line in its own Spans
        // Makes RequestError::Deserialisation present far more nicely
        for line in error.to_string().lines() {
            text.push(Line::from(line.to_owned()));
        }

        Paragraph::new(text)
            .block(Block::default().title("[Uh oh]").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
    }

    fn format_list<S: Display>(strings: &[S]) -> String {
        match strings.len() {
            0 => String::new(),
            1 => strings[0].to_string(),
            2 => format!("{} and {}", strings[0], strings[1]),
            _ => {
                let last_index = strings.len() - 1;
                let mut buf = strings[..last_index].iter().join(", ");
                // Oxford comma hell yeah
                buf.push_str(", and ");
                buf.push_str(&strings[last_index].to_string());
                buf
            },
        }
    }

    #[cfg(test)]
    mod unit_tests {
        use super::format_list;

        #[test]
        fn correct_lists() {
            // 0 elements
            let empty: [&str; 0] = [];
            assert_eq!(format_list(&empty), String::new());

            // 1 element
            let e = "e";
            assert_eq!(format_list(&[e]), String::from(e));

            // 2 elements
            let two = ["one", "two"];
            let output = format_list(&two);
            assert!(
                !output.contains(','),
                "two item list shouldn't contain commas",
            );
            for item in two {
                assert!(output.contains(item), "missing {item} in list");
            }

            // 3 elements
            let three = ["one", "two", "three"];
            let output = format_list(&three);
            for item in three {
                assert!(output.contains(item), "missing {item} in list");
            }
        }
    }
}
