use crate::*;
use crossterm::cursor::MoveToPreviousLine;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType::FromCursorDown;
use crossterm::ExecutableCommand;
use requestty::question::Choice;
use requestty::Question;
use std::cmp::min;
use std::fmt::{Debug, Display, Formatter};
use std::io::stdout;
use std::ops::Rem;

const PAGE_MAX: usize = 25;
const NEXT_PAGE_LABEL: &str = "Next page";
const PREV_PAGE_LABEL: &str = "Previous page";
const GIVE_UP_LABEL: &str = "I can't see what I'm looking for";

pub fn get_search_term() -> Result<String> {
    let question = Question::input("search_term")
        .message("Please enter the name of the movie/show you're looking for")
        .build();
    Ok(requestty::prompt_one(question)?.try_into_string().unwrap())
}

pub struct Pager<'a, E> {
    choices: Vec<Choice<String>>,
    entries: &'a [E],
    page_size: usize,
    page_index: usize,
    max_page_index: usize,
}

impl<'a, E> Pager<'a, E>
where
    E: Display,
{
    pub fn new(search_results: &'a [E], config: &RuntimeConfig) -> Self {
        let page_size = min(config.number_of_results, PAGE_MAX);

        let choices = search_results
            .iter()
            .map(|sr| sr.to_string().into())
            .collect();

        let mut max_page_index = search_results.len() / page_size;
        if search_results.len().rem(page_size) == 0 {
            max_page_index -= 1;
        }
        Pager {
            choices,
            entries: search_results,
            page_size,
            page_index: 0,
            max_page_index,
        }
    }

    pub fn ask(&mut self) -> Result<&'a E> {
        let mut stdout = stdout();
        loop {
            let start_index = self.page_index * self.page_size;
            let end_index = min(start_index + self.page_size, self.choices.len());
            let mut displayed_choices = self.choices[start_index..end_index].to_vec();
            let results_being_shown = displayed_choices.len();
            displayed_choices.push(requestty::DefaultSeparator);

            if self.page_index < self.max_page_index {
                displayed_choices.push(Choice::Choice(NEXT_PAGE_LABEL.into()));
            }
            if self.page_index > 0 {
                displayed_choices.push(Choice::Choice(PREV_PAGE_LABEL.into()));
            }

            displayed_choices.push(Choice::Choice(GIVE_UP_LABEL.into()));

            let question = Question::select("")
                .message(if self.page_index == 0 {
                    String::from("Pick the correct search result")
                } else {
                    format!(
                        "Page {} of {}",
                        self.page_index + 1,
                        self.max_page_index + 1
                    )
                })
                .choices(displayed_choices)
                .build();

            let answer = requestty::prompt_one(question)?;
            let list_item = answer.as_list_item().unwrap();
            if list_item.index < results_being_shown {
                // Chose one of the search results
                return Ok(self.entries.get(start_index + list_item.index).unwrap());
            } else {
                // Work out which other option has been chosen
                stdout
                    .execute(MoveToPreviousLine(1))?
                    .execute(Clear(FromCursorDown))?;
                match list_item.text.as_str() {
                    NEXT_PAGE_LABEL => self.next_page(),
                    PREV_PAGE_LABEL => self.prev_page(),
                    GIVE_UP_LABEL => return Err(RunError::NoDesiredSearchResults),
                    other => unreachable!("Please raise an issue because you managed to choose an option I didn't expect ({})", other),
                }
            }
        }
    }

    fn next_page(&mut self) {
        self.page_index = min(self.page_index + 1, self.max_page_index);
    }

    fn prev_page(&mut self) {
        self.page_index = self.page_index.saturating_sub(1);
    }
}

impl<'a, E> Debug for Pager<'a, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Search results: {}\n", self.entries.len())?;
        write!(f, "Page size: {}\n", self.page_size)?;
        write!(f, "Number of pages: {}\n", self.max_page_index + 1)?;
        write!(f, "Page index: {}\n", self.page_index)
    }
}

// TODO: test?
