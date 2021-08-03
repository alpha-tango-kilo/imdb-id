use std::{fmt, io};

use requestty::ErrorKind;
use std::error::Error;
use std::num::ParseIntError;
use RunError::*;
use SearchResultWarning::*;

/*
Variants prefixed with "Clap" will be printed by Clap as such:
error: Invalid value for '<arg>': <YOUR MESSAGE>
 */

pub type Result<T> = std::result::Result<T, RunError>;

#[derive(Debug)]
pub enum RunError {
    ClapNotUsize,
    InvalidYearRange(ParseIntError),
    ClapInvalidFormat,
    ClapMissingFeature(&'static str), // required feature(s)
    NoSearchResults,
    Reqwest(reqwest::Error),
    InputUserHalted,
    InputIo(io::Error), // includes crossterm
    NoDesiredSearchResults,
}

impl RunError {
    pub fn error_code(&self) -> i32 {
        /*
        0 for success
        1 for user error
        2 for program error
         */
        match self {
            ClapNotUsize => 1,
            InvalidYearRange(_) => 1,
            ClapInvalidFormat => 1,
            ClapMissingFeature(_) => 1,
            NoSearchResults => 1,
            Reqwest(_) => 2,
            InputUserHalted => 1,
            InputIo(_) => 2,
            NoDesiredSearchResults => 0,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClapNotUsize => write!(f, "expected a positive integer"),
            InvalidYearRange(err) => write!(f, "Invalid year / year range ({})", err),
            ClapInvalidFormat => write!(
                f,
                "invalid format\nIf you think this should have \
            worked, please ensure you installed the tool with the required features\n\
            See the project README for more information"
            ),
            ClapMissingFeature(fs) => write!(f, "missing feature(s) {} for that operation", fs),
            NoSearchResults => write!(f, "No search results"),
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

#[derive(Debug)]
pub enum SearchResultWarning {
    ImdbIdNotFound(String),
    NameNotFound(String),
}

impl fmt::Display for SearchResultWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImdbIdNotFound(s) => write!(f, "IMDb ID not found, please raise an issue if you are able to see the ID in the following text: {:?}", s),
            NameNotFound(s) => write!(f, "Movie/Show name not found, please raise an issue if you are able to see a name in the following text: {:?}", s),
        }
    }
}

impl Error for SearchResultWarning {}
