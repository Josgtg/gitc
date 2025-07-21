use std::io::{BufRead, Cursor};

use anyhow::{bail, Context, Result};

pub trait EasyRead {
    fn read_until(&mut self) -> Result<Vec<u8>>;
}

impl<T: AsRef<[u8]>> EasyRead for Cursor<T> {
    /// This function is just an abstraction to simplify other functions since this process is used
    /// a lot.
    ///
    /// It already handles the errors and returns them with context, so it can just be handled with
    /// the `?` operator.
    fn read_until(byte: u8, cursor: &mut Cursor<T>) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        cursor
            .read_until(byte, &mut buf)
            .context(format!("could not read until {}", byte))?;
        if buf.pop() != Some(byte) {
            bail!("expected {}", byte)
        }
        Ok(buf)
    }
}
