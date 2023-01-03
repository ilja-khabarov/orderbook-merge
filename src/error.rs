use futures_util::future::err;
use std::num::{ParseFloatError, ParseIntError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneralError {
    /// Kind of error we can't do anything with, but not fatal for our service
    #[error("DataError: {0}")]
    DataError(String),
    #[error("ParseError: {0}")]
    ParseError(String),
    #[error("Error message not provided")]
    UndefinedError,
}

impl GeneralError {
    pub(crate) fn orders_format_error() -> Self {
        Self::DataError("Exchange provided inconsistent data".to_string())
    }
    pub(crate) fn parse_error() -> Self {
        Self::DataError("Exchange provided inconsistent data".to_string())
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

pub type OrderbookResult<T> = core::result::Result<T, GeneralError>;
