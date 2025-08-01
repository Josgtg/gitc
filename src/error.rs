use std::fmt::Debug;

pub trait WarnUnwrap<T, E> {
    fn warn_unwrap_or_default(self) -> T where T: Default;
    #[allow(unused)]
    fn warn_unwrap_or(self, default: T) -> T;
    #[allow(unused)]
    fn warn_unwrap_match<V>(self, ok: fn(T) -> V, err: V) -> V;
}

impl<T, E: Debug> WarnUnwrap<T, E> for Result<T, E> {

    /// Tries to unwrap a `Result`, if it was the `Err` variant, logs the error and returns the default
    /// value of the type `T`.
    fn warn_unwrap_or_default(self) -> T where T: Default {
        match self {
            Ok(value) => value,
            Err(error) => {
                log::warn!("{:?}", error);
                T::default()
            }
        }
    }

    /// Tries to unwrap a `Result`, if it was the `Err` variant, logs the error and returns the
    /// provided default.
    fn warn_unwrap_or(self, default: T) -> T {
        match self {
            Ok(value) => value,
            Err(error) => {
                log::warn!("{:?}", error);
                default
            }
        }
    }

    /// Tries to unwrap a `Result`:
    /// - If the value was the `Ok` variant, returns the result of a function using the inner value.
    /// - It it was the `Err` variant, it logs it and returns `err`.
    fn warn_unwrap_match<V>(self, ok: fn(T) -> V, err: V) -> V {
        match self {
            Ok(value) => ok(value),
            Err(error) => {
                log::warn!("{:?}", error);
                err
            }
        }
    }
}
