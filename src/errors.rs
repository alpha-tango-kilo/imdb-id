use std::{fmt, io};

use requestty::ErrorKind;
use std::error::Error;
use std::num::ParseIntError;
use RunError::*;

/*
Variants prefixed with "Clap" will be printed by Clap as such:
error: Invalid value for '<arg>': <YOUR MESSAGE>
 */

pub type Result<T> = std::result::Result<T, RunError>;

#[derive(Debug)]
pub enum RunError {
    Clap(ClapError),
    InvalidYearRange(ParseIntError),
    NoSearchResults,
    Reqwest(reqwest::Error),
    InputUserHalted,
    InputIo(io::Error), // includes crossterm
    NoDesiredSearchResults,
    Serde(Box<dyn Error>),
    OmdbNotFound(String),                        // search term
    OmdbUnrecognised(String, serde_json::Error), // raw response JSON
}

impl RunError {
    pub fn error_code(&self) -> i32 {
        /*
        0 for success
        1 for user error
        2 for program error
         */
        match self {
            Clap(_) => 1,
            InvalidYearRange(_) => 1,
            NoSearchResults => 1,
            Reqwest(_) => 2,
            InputUserHalted => 1,
            InputIo(_) => 2,
            NoDesiredSearchResults => 0,
            Serde(_) => 2,
            OmdbNotFound(_) => 1,
            OmdbUnrecognised(_, _) => 2,
        }
    }
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Clap(clap_err) => write!(f, "Argument parsing problem: {}", clap_err),
            InvalidYearRange(err) => write!(f, "Invalid year / year range: {}", err),
            NoSearchResults => write!(f, "No search results"),
            Reqwest(reqwest_err) => write!(f, "Issue with web request: {}", reqwest_err),
            InputUserHalted => write!(f, "Program halted at user request"),
            InputIo(io_err) => write!(f, "IO error: {}", io_err),
            NoDesiredSearchResults => write!(f, "You couldn't find what you wanted :("),
            Serde(e) => write!(f, "Failed to serialise output data: {}", e),
            OmdbNotFound(search_term) => write!(f, "No record found on OMDb for {:?}", search_term),
            OmdbUnrecognised(json, err) => write!(
                f,
                "Unrecognised response from OMDb, please raise an issue including the following text:\n\
                Serde error: \n\
                ```\n\
                {}\n\
                ```\n\
                JSON: \n\
                ```\n\
                {}\n\
                ```",
                err, json
            ),
        }
    }
}

impl Error for RunError {}

impl From<ClapError> for RunError {
    fn from(clap_err: ClapError) -> Self {
        Clap(clap_err)
    }
}

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

impl From<serde_json::Error> for RunError {
    fn from(ser_err: serde_json::Error) -> Self {
        Serde(Box::new(ser_err))
    }
}

#[cfg(feature = "yaml")]
impl From<serde_yaml::Error> for RunError {
    fn from(ser_err: serde_yaml::Error) -> Self {
        Serde(Box::new(ser_err))
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ClapError {
    NotUsize,
    InvalidFormat,
}

impl fmt::Display for ClapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ClapError::*;
        match self {
            NotUsize => write!(f, "expected a positive integer"),
            InvalidFormat => write!(
                f,
                "invalid format\nIf you think this should have \
            worked, please ensure you installed the tool with the required features\n\
            See the project README for more information"
            ),
        }
    }
}

impl Error for ClapError {}
