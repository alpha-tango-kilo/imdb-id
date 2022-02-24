use std::error::Error;
use std::fmt::Display;
use std::io;
use std::num::ParseIntError;
use thiserror::Error;

// To be implemented on types that contain some non-fatal errors and wish to
// take advantage of EmitNonFatal
pub trait MaybeFatal {
    fn is_fatal(&self) -> bool {
        false
    }
}

pub trait EmitNonFatal<E> {
    fn emit_non_fatal(self) -> Result<(), E>;
    fn emit_unconditional(self);
}

impl<E: MaybeFatal + Display> EmitNonFatal<E> for E {
    fn emit_non_fatal(self) -> Result<(), E> {
        if self.is_fatal() {
            Err(self)
        } else {
            eprintln!("WARNING: {self}");
            Ok(())
        }
    }

    fn emit_unconditional(self) {
        if self.is_fatal() {
            panic!("emit_unconditional called on fatal error: {self}");
        } else {
            eprintln!("WARNING: {self}");
        }
    }
}

impl<E: MaybeFatal + Display> EmitNonFatal<E> for Result<(), E> {
    fn emit_non_fatal(self) -> Result<(), E> {
        match self {
            Ok(()) => Ok(()),
            Err(e) => {
                if e.is_fatal() {
                    Err(e)
                } else {
                    eprintln!("WARNING: {e}");
                    Ok(())
                }
            }
        }
    }

    fn emit_unconditional(self) {
        if let Err(e) = self {
            if e.is_fatal() {
                panic!("emit_unconditional called on fatal error: {e}");
            } else {
                eprintln!("WARNING: {e}");
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum FinalError {
    #[error("invalid commandline argument: {0}")]
    Args(#[from] ArgsError),
    #[error(transparent)]
    Interaction(#[from] InteractivityError),
    #[error("no search results :(")]
    NoSearchResults,
    #[error("failed to format output as requested: {0}")]
    FormatOutput(Box<dyn Error>),
}

impl FinalError {
    pub fn error_code(&self) -> i32 {
        use FinalError::*;
        /*
        0 for success
        1 for user error
        2 for program error
         */
        match self {
            Args(_) => 1,
            // 0 if non-fatal, 2 if fatal
            Interaction(inner) => (inner.is_fatal() as i32) * 2,
            NoSearchResults => 0,
            FormatOutput(_) => 2,
        }
    }
}

impl MaybeFatal for FinalError {
    fn is_fatal(&self) -> bool {
        use FinalError::*;
        match self {
            Interaction(inner) => inner.is_fatal(),
            _ => true,
        }
    }
}

impl From<serde_json::Error> for FinalError {
    fn from(err: serde_json::Error) -> Self {
        FinalError::FormatOutput(Box::new(err))
    }
}

#[cfg(feature = "yaml")]
impl From<serde_yaml::Error> for FinalError {
    fn from(err: serde_yaml::Error) -> Self {
        FinalError::FormatOutput(Box::new(err))
    }
}

#[derive(Debug, Error)]
pub enum ArgsError {
    #[error("bad number of results: {0}")]
    NumberOfResults(#[from] ParseIntError),
    #[error("bad year: {0}")]
    NotYear(#[from] YearParseError),
    #[error("bad output format: {0}")]
    OutputFormat(#[from] OutputFormatParseError),
    #[error(transparent)]
    MediaType(#[from] MediaTypeParseError),
    #[error(transparent)]
    SearchTerm(#[from] InteractivityError),
}

/*
Can't derive this because we don't want to inspect into InteractivityError we
don't want to derive PartialEq for InteractivityError because io::Error doesn't
implement it, thus rendering the entire exercise useless, so we might as well
deal with it at this high level
 */
#[cfg(test)]
impl PartialEq for ArgsError {
    fn eq(&self, other: &Self) -> bool {
        use ArgsError::*;
        match (self, other) {
            (NumberOfResults(a), NumberOfResults(b)) => a == b,
            (NotYear(a), NotYear(b)) => a == b,
            (OutputFormat(a), OutputFormat(b)) => a == b,
            (MediaType(a), MediaType(b)) => a == b,
            (SearchTerm(_), SearchTerm(_)) => true,
            _ => false,
        }
    }
}

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum OutputFormatParseError {
    #[error("this format isn't supported because you didn't enable it at compile time.\nYou can 'enable' this by running `cargo install imdb-id --force --features {0}`")]
    NotInstalled(String),
    #[error("{0:?} is not a recognised output format")]
    Unrecognised(String),
}

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum YearParseError {
    #[error(transparent)]
    InvalidInt(#[from] ParseIntError),
    #[error("no year was specified at either end of the range")]
    NoYearsSpecified,
    #[error("start of date range is in the future")]
    StartInFuture,
}

#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
#[error("unrecognised media type {0:?}")]
pub struct MediaTypeParseError(pub String);

#[derive(Debug, Error)]
pub enum InteractivityError {
    #[error("user aborted operation")]
    Cancel,
    #[error("unexpected CLI error: {0}\nIf you were just trying to stop running the program, please create an issue about this")]
    Dialoguer(io::Error),
    #[error("unexpected crossterm error: {0}")]
    Crossterm(io::Error),
    #[error("unexpected TUI error: {0}")]
    Tui(io::Error),
}

impl MaybeFatal for InteractivityError {
    fn is_fatal(&self) -> bool {
        matches!(self, InteractivityError::Dialoguer(_))
    }
}

impl InteractivityError {
    pub fn from_cli(err: io::Error) -> Self {
        use InteractivityError::*;
        match err.kind() {
            io::ErrorKind::NotConnected => Cancel,
            _ => Dialoguer(err),
        }
    }
}

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("issue with request: {0}")]
    Web(#[from] minreq::Error),
    #[error("unrecognised response from OMDb, please raise an issue including the following text:\nSerde error: {0}\nJSON: \n```\n{1}\n```")]
    Deserialisation(serde_json::Error, String),
    #[error("OMDb gave us an error: {0}")]
    Omdb(String),
}

#[derive(Debug, Error)]
pub enum SignUpError {
    #[error(transparent)]
    Interactivity(#[from] InteractivityError),
    #[error(transparent)]
    MinReq(#[from] minreq::Error),
    #[error("response didn't indicate success")]
    NeedleNotFound,
}

impl MaybeFatal for SignUpError {
    fn is_fatal(&self) -> bool {
        use SignUpError::*;
        match self {
            Interactivity(inner) => inner.is_fatal(),
            MinReq(_) => true,
            NeedleNotFound => false,
        }
    }
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

// Always printed "WARNING: {DiskError}", as these are never fatal errors
#[derive(Debug, Error)]
pub enum DiskError {
    #[error("config file does not exist at {0}")] // this is never seen
    NotFound(&'static str), // path (converted lossy)
    #[error("failed to read saved config: {0}")]
    Read(io::Error),
    #[error("failed to interpret saved config at {1}: {0}")]
    Deserialise(#[source] serde_json::Error, &'static str), // path (converted lossy)
    #[error("failed to save config: {0}")]
    Write(io::Error),
    #[error("failed to convert config to JSON for writing: {0}")]
    Serialise(serde_json::Error),
}

impl MaybeFatal for DiskError {}
