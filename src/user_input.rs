use crate::omdb::test_api_key;
use crate::Result;
use crate::RunError::NoDesiredSearchResults;
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use lazy_static::lazy_static;
use std::fmt::Display;
use std::ops::Deref;

lazy_static! {
    static ref THEME: ColorfulTheme = Default::default();
}

pub fn get_api_key() -> Result<String> {
    let api_key = Input::with_theme(THEME.deref())
        .with_prompt("Please enter in your OMDb API key. If you need to, visit their website to get one (https://www.omdbapi.com/apikey.aspx)")
        .validate_with(|api_key: &String| test_api_key(api_key))
        .interact_text()?;
    Ok(api_key)
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
        .ok_or(NoDesiredSearchResults)
}
