use std::error::Error;
use std::io;
use std::num::ParseIntError;
use thiserror::Error;
use RunError::*;

pub type Result<T, E = RunError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum RunError {
    #[error("Argument parsing problem: {0}")]
    Clap(#[from] ClapError),
    #[error("Unsupported media type: {0}")]
    InvalidMediaType(String),
    #[error("Invalid year / year range: {0}")]
    InvalidYearRange(#[from] YearParseError),
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
    #[error("Unrecognised response from OMDb, please raise an issue including the following text:\nSerde error: {1}\nJSON: \n```\n{0}\n```")]
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
            InvalidMediaType(_) => 1,
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

#[derive(Debug, Error)]
pub enum YearParseError {
    #[error(transparent)]
    InvalidInt(#[from] ParseIntError),
    #[error("no year was specified at either end of the range")]
    NoYearsSpecified,
}

#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error("Invalid API key format")]
    InvalidFormat,
    #[error("Issue with web request: {0}")]
    RequestFailed(#[from] minreq::Error),
    #[error("Unauthorised API key")]
    Unauthorised,
    #[error("Unexpected response: status {0}")]
    UnexpectedStatus(i32),
}

#[derive(Debug, Error)]
pub enum SignUpError {
    #[error(transparent)]
    Dialoguer(#[from] io::Error),
    #[error(transparent)]
    MinReq(#[from] minreq::Error),
    #[error("response didn't indicate success")]
    NeedleNotFound,
}
