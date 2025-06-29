use std::rc::Rc;

/// Enum intended to represent all the different error types that there could be
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("method is not implemented")]
    NotImplemented,
    #[error("there was an error during the execution of the program")]
    Generic(Rc<str>),
    #[error("operation could not be completed: {0}")]
    Operation(Rc<str>),
    #[error("i/o operation error: {0:?}")]
    IO(#[from] std::io::Error),
    #[error("utf-8 encoding error: {0:?}")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("there was an error with data formatting: {0}")]
    Formatting(Rc<str>),
    #[error("there is inconsistent data: {0}")]
    DataConsistency(Rc<str>),
    #[error("argument {0:?} is not valid")]
    Arg(Rc<str>),
    #[error("error working with time")]
    SystemTime(#[from] std::time::SystemTimeError),
    #[error("error stripping prefix from a file")]
    StripPrefix(#[from] std::path::StripPrefixError),
}

/// Abstraction of the result type where the error is always an Error from this crate.
pub type Result<T, E = Error> = core::result::Result<T, E>;
