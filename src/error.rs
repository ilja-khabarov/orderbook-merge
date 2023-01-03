use thiserror::Error;

#[derive(Error, Debug)]
pub enum GeneralError {
    /// Kind of error we can't do anything with, but not fatal for our service
    #[error("DataError: {0}")]
    DataError(String),
    #[error("Error message not provided")]
    UndefinedError,
}

impl GeneralError {
    pub(crate) fn orders_format_error() -> Self {
        Self::DataError("Exchange provided inconsistent data".to_string())
    }
}

pub type Result<T> = core::result::Result<T, GeneralError>;
