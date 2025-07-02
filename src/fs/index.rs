use std::fs;
use std::io::Cursor;

use crate::byteable::Byteable;
use crate::error::CustomResult;
use crate::index::Index;
use crate::{Constants, Result};

pub fn read_index_file() -> Result<Index> {
    let index_path = Constants::index_path();

    // returning empty index entry
    if !fs::exists(&index_path).map_err_with("could not check index file existance")? {
        return Ok(Index::default());
    }

    let data = fs::read(index_path).map_err_with("could not read index file data")?;
    let mut cursor = Cursor::new(data);

    let index = Index::from_bytes(&mut cursor)
        .map_err_with("could not create index from index file's data")?;

    Ok(index)
}

pub fn write_index_file(index: Index) -> Result<()> {
    let data = index
        .as_bytes()
        .map_err_with("could not encode index when trying to write to index file")?;

    fs::write(Constants::index_path(), data).map_err_with("could not write data to index file")?;

    Ok(())
}
