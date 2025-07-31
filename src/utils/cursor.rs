use std::io::BufRead;
use std::io::Cursor;

use anyhow::{Context, Result, bail};

pub trait EasyRead {
    fn read_until_checked(&mut self, byte: u8) -> Result<Vec<u8>>;
}

impl<T: AsRef<[u8]>> EasyRead for Cursor<T> {
    /// This function is just an abstraction to simplify other functions since this process is used
    /// a lot.
    ///
    /// It already handles the errors (not reading until expected byte or not reading at all)
    /// and returns them with context, so it can just be handled with the `?` operator.
    fn read_until_checked(&mut self, byte: u8) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        BufRead::read_until(self, byte, &mut buf)
            .context(format!("could not read until {}", byte))?;
        if buf.pop() != Some(byte) {
            bail!("expected {}", byte)
        }
        Ok(buf)
    }
}
