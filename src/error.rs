use std::fmt;

use std::error::Error;
use RunError::*;

/*
Variants prefixed with "Clap" will be printed by Clap as such:
error: Invalid value for '<arg>': <YOUR MESSAGE>
 */

#[derive(Debug)]
pub enum RunError {
    ClapNotUsize,
    ImdbIdNotFound(String),
    NameNotFound(String),
    Reqwest(reqwest::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClapNotUsize => write!(f, "expected a positive integer"),
            ImdbIdNotFound(s) => write!(f, "IMDb ID not found, please raise an issue if you are able to see the ID in the following text: {:?}", s),
            NameNotFound(s) => write!(f, "Movie/Show name not found, please raise an issue if you are able to see a name in the following text: {:?}", s),
            Reqwest(reqwest_err) => write!(f, "Issue with web request: {}", reqwest_err),
        }
    }
}

impl Error for RunError {}

impl From<reqwest::Error> for RunError {
    fn from(reqwest_err: reqwest::Error) -> Self {
        Reqwest(reqwest_err)
    }
}
