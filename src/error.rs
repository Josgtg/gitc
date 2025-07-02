use std::rc::Rc;

/// Enum intended to represent all the different error types that there could be
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("method is not implemented")]
    NotImplemented,
    #[error("{message}\ncaused by: {source:?}")]
    Custom {
        message: Rc<str>,
        #[source]
        source: Box<Error>,
    },
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

impl Error {
    pub fn print_backtrace(&self) {
        let mut err = self;
        if let Self::Custom { message, source } = err {
            eprintln!("error: {message}");
            err = source;
            while let Self::Custom { message, source } = err {
                eprintln!("caused by: {message}");
                err = source;
            }
            eprintln!("caused by: {err:?}");
        } else {
            eprintln!("{err:?}");
        }
    }
}

pub trait CustomError {
    fn custom(self, message: impl AsRef<str>) -> Error
    where
        Self: Into<Error>;
}

impl<E: Into<Error>> CustomError for E {
    fn custom(self, message: impl AsRef<str>) -> Error {
        Error::Custom {
            message: message.as_ref().into(),
            source: Box::new(self.into()),
        }
    }
}

pub trait CustomResult<T> {
    fn map_err_with(self, message: impl AsRef<str>) -> Result<T>;
}

impl<T, E: Into<Error>> CustomResult<T> for core::result::Result<T, E> {
    /// Adds a context message to this error.
    fn map_err_with(self, message: impl AsRef<str>) -> Result<T> {
        self.map_err(|e| e.custom(message))
    }
}
