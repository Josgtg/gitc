use std::str::FromStr;

use crate::fs;
use crate::hashing::Hash;

use anyhow::{Context, Result};

pub fn cat_file(hash: &str) -> Result<String> {
    // just checking the string is valid
    let hash = Hash::from_str(hash).context("object hash was invalid")?;

    let object = fs::object::read_object(hash).context("could not read object")?;

    Ok(format!("{}", object))
}
