use std::{fmt, io};

use requestty::ErrorKind;
use std::error::Error;
use RunError::*;

/*
Variants prefixed with "Clap" will be printed by Clap as such:
error: Invalid value for '<arg>': <YOUR MESSAGE>
 */

pub type Result<T> = std::result::Result<T, RunError>;

#[derive(Debug)]
pub enum RunError {
    ClapNotUsize,
    ImdbIdNotFound(String),
    NameNotFound(String),
    Reqwest(reqwest::Error),
    InputUserHalted,
    InputIo(io::Error),
    NoDesiredSearchResults,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClapNotUsize => write!(f, "expected a positive integer"),
            ImdbIdNotFound(s) => write!(f, "IMDb ID not found, please raise an issue if you are able to see the ID in the following text: {:?}", s),
            NameNotFound(s) => write!(f, "Movie/Show name not found, please raise an issue if you are able to see a name in the following text: {:?}", s),
            Reqwest(reqwest_err) => write!(f, "Issue with web request: {}", reqwest_err),
            InputUserHalted => write!(f, "Program halted at user request"),
            InputIo(io_err) => write!(f, "IO error: {}", io_err),
            NoDesiredSearchResults => write!(f, "You couldn't find what you wanted :("),
        }
    }
}

impl Error for RunError {}

impl From<reqwest::Error> for RunError {
    fn from(reqwest_err: reqwest::Error) -> Self {
        Reqwest(reqwest_err)
    }
}

impl From<requestty::ErrorKind> for RunError {
    fn from(requestty_err: ErrorKind) -> Self {
        use requestty::ErrorKind::*;
        match requestty_err {
            Eof | Interrupted => InputUserHalted,
            IoError(io_err) => Self::from(io_err),
        }
    }
}

impl From<io::Error> for RunError {
    fn from(io_err: io::Error) -> Self {
        InputIo(io_err)
    }
}
