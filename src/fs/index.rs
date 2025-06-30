use std::fs;
use std::io::Cursor;

use crate::byteable::Byteable;
use crate::index::Index;
use crate::{Constants, Result};

pub fn read_index_file() -> Result<Index> {
    let data = fs::read(Constants::index_path())?;
    let mut cursor = Cursor::new(data);

    let index = Index::from_bytes(&mut cursor)?;

    Ok(index)
}

pub fn write_index_file(index: Index) -> Result<()> {
    let data = index.as_bytes()?;

    fs::write(Constants::index_path(), data)?;

    Ok(())
} 
