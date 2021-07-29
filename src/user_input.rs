use crate::*;
use requestty::question::Choice;
use requestty::Question;
use std::cmp::min;
use std::fmt::{Debug, Formatter};

const PAGE_MAX: usize = 25;

pub fn get_search_term() -> Result<String> {
    let question = Question::input("search_term")
        .message("Please enter the name of the movie/show you're looking for")
        .build();
    Ok(requestty::prompt_one(question)?.try_into_string().unwrap())
}

pub struct Pager<'a> {
    choices: Vec<Choice<String>>,
    search_results: &'a Vec<SearchResult>,
    page_size: usize,
    page_index: usize,
    max_page_index: usize,
}

impl<'a> Pager<'a> {
    pub fn new(search_results: &'a Vec<SearchResult>, config: &RuntimeConfig) -> Self {
        // Subtract 3 to account for separator and misc. options
        let page_size = min(config.number_of_results, PAGE_MAX - 3);
        let choices = search_results
            .iter()
            .map(|sr| sr.to_string().into())
            .collect();
        Pager {
            choices,
            search_results,
            page_size,
            page_index: 0,
            max_page_index: search_results.len() / page_size,
        }
    }

    pub fn ask(&mut self) -> Result<&'a SearchResult> {
        loop {
            let start_index = self.page_index * self.page_size;
            let end_index = min(start_index + self.page_size, self.choices.len());
            let there_is_another_page = self.page_index < self.max_page_index;
            let mut choices = self.choices[start_index..end_index].to_vec();
            let results_being_shown = choices.len();
            choices.push(requestty::DefaultSeparator);
            if there_is_another_page {
                choices.push(Choice::Choice("Next page".into()));
            }
            choices.push(Choice::Choice("I can't see what I'm looking for".into()));
            let number_of_choices = choices.len();

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
                .choices(choices)
                .build();

            let answer = requestty::prompt_one(question)?;
            let list_item = answer.as_list_item().unwrap();
            if list_item.index < results_being_shown {
                // Chose one of the search results
                return Ok(self
                    .search_results
                    .get(start_index + list_item.index)
                    .unwrap());
            } else if list_item.index == number_of_choices - 2 {
                // The above condition shouldn't be hit if there's no next page
                // Next page
                self.next_page();
                continue;
            } else if list_item.index == number_of_choices - 1 {
                // Give up
                return Err(RunError::NoDesiredSearchResults);
            } else {
                // Selector chosen
                unreachable!("requestty let you select the separator. Please raise an issue");
            }
        }
    }

    fn next_page(&mut self) {
        self.page_index = min(self.page_index + 1, self.max_page_index);
    }

    /*
    fn prev_page(&mut self) {
        self.page_index = self.page_size.saturating_sub(1);
    }
    */
}

impl<'a> Debug for Pager<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Search results: {}\n", self.search_results.len())?;
        write!(f, "Page size: {}\n", self.page_size)?;
        write!(f, "Number of pages: {}\n", self.max_page_index)?;
        write!(f, "Page index: {}\n", self.page_index)
    }
}
