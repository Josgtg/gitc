use std::fs;

use anyhow::{Context, Result};

use crate::Constants;
use crate::byteable::Byteable;
use crate::index::Index;

pub fn read_index_file() -> Result<Index> {
    let index_path = Constants::index_path();

    // returning empty index entry
    if !fs::exists(&index_path).context("could not check index file existance")? {
        return Ok(Index::default());
    }

    let data = fs::read(index_path).context("could not read index file data")?;

    let index =
        Index::from_bytes(&data).context("could not create index from index file's data")?;

    Ok(index)
}

pub fn write_index_file(index: Index) -> Result<()> {
    let data = index
        .as_bytes()
        .context("could not encode index when trying to write to index file")?;

    fs::write(Constants::index_path(), data).context("could not write data to index file")?;

    Ok(())
}
