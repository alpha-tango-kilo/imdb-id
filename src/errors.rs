use std::error::Error;
use std::num::ParseIntError;
use thiserror::Error;
use RunError::*;

pub type Result<T, E = RunError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Argument parsing problem: {0}")]
    Clap(#[from] ClapError),
    #[error("Unsupported genre: {0}")]
    InvalidGenre(String),
    #[error("Invalid year / year range: {0}")]
    InvalidYearRange(#[from] ParseIntError),
    #[error("No search results")]
    NoSearchResults,
    #[error("Issue with web request: {0}")]
    MinReq(#[from] minreq::Error),
    #[error("IO error: {0}")]
    InputIo(#[from] std::io::Error),
    #[error("You couldn't find what you wanted :(")]
    NoDesiredSearchResults,
    #[error("Failed to serialise output data: {0}")]
    Serde(#[source] Box<dyn Error>),
    #[error("No record found on OMDb for {0:?}")]
    OmdbNotFound(String), // search term
    #[error("OMDb API returned an error: {0:?}")]
    OmdbError(String), // "Error" field of response
    #[error("Unrecognised response from OMDb, please raise an issue including the following text:\nSerde error: {0}\nJSON: \n```\n{1}\n```")]
    OmdbUnrecognised(String, #[source] serde_json::Error), // raw response JSON
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
            InvalidGenre(_) => 1,
            InvalidYearRange(_) => 1,
            NoSearchResults => 1,
            MinReq(_) => 2,
            InputIo(_) => 2,
            NoDesiredSearchResults => 0,
            Serde(_) => 2,
            OmdbNotFound(_) => 1,
            OmdbError(_) => 3,
            OmdbUnrecognised(_, _) => 2,
        }
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

/*
Will be printed by Clap as such:
error: Invalid value for '<arg>': <YOUR MESSAGE>
 */

#[derive(Debug, Error)]
pub enum ClapError {
    #[error("expected a positive integer")]
    NotUsize,
    #[error("invalid format\nIf you think this should have worked, please ensure you installed the tool with the required features\nSee the project README for more information")]
    InvalidFormat,
}
