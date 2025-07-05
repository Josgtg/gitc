use anyhow::{Context, Result};

use crate::fs;

pub fn ls_files(debug: bool) -> Result<String> {
    let index = fs::index::read_index_file().context("could not read from index file")?;

    if index.entries().count() == 0 {
        return Ok(String::new());
    }

    let mut formatted = String::new();
    for e in index.entries() {
        if debug {
            formatted.push_str(format!("{e:?}").as_str());
            formatted.push('\n');
        } else {
            formatted.push_str(format!("{e}").as_str());
        }
        formatted.push('\n');
    }
    formatted.pop();
    if debug {
        formatted.pop();
    } // removing extra new line

    Ok(formatted.to_string())
}
