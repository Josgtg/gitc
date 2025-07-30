use std::fmt::Debug;
use log::warn;

pub trait WarnUnwrap<T> {
    fn warn_unwrap(self) -> T;
}

impl<T: Default, E: Debug> WarnUnwrap<T> for Result<T, E> {
    /// Tries to unwrap a value, if the result was the Err variant, it logs the error and
    /// returns a default value.
    fn warn_unwrap(self) -> T {
        match self {
            Ok(value) => value,
            Err(error) => {
                warn!("{:?}", error);
                T::default()
            }
        }
    }
}
