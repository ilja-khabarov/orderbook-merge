use std::num::{ParseFloatError, ParseIntError};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum GeneralError {
    /// Kind of error we can't do anything with, but not fatal for our service
    #[error("DataError: {0}")]
    DataError(String),
    #[error("ParseError: {0}")]
    ParseError(String),
    #[error("ConnectionError: {0}")]
    ConnectionError(String),
}

impl GeneralError {
    pub(crate) fn orders_format_error() -> Self {
        Self::DataError("Exchange provided inconsistent data".to_string())
    }
    pub(crate) fn connection_error(error_message: String) -> Self {
        Self::ConnectionError(error_message)
    }
}

impl From<ParseIntError> for GeneralError {
    fn from(error: ParseIntError) -> Self {
        Self::ParseError(format!("{}", error))
    }
}

impl From<ParseFloatError> for GeneralError {
    fn from(error: ParseFloatError) -> Self {
        Self::ParseError(format!("{}", error))
    }
}

impl From<serde_json::Error> for GeneralError {
    fn from(error: serde_json::Error) -> Self {
        Self::ParseError(format!("{}", error))
    }
}

pub(crate) type OrderbookResult<T> = Result<T, GeneralError>;
