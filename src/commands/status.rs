use std::path::PathBuf;

use colored::Colorize;

use crate::error::CustomResult;
use crate::object::Object;
use crate::{Constants, Result};

use crate::fs::index::read_index_file;

type ObjectData = (PathBuf, Object);

pub fn status() -> Result<String> {
    let mut status = String::new();
    let index = read_index_file().map_err_with("could not read from index file")?;

    let files = search_dir(Constants::repository_folder_path())?;
    
    for (ie, (path, object)) in index.entries().zip(files) {
        if ie.path().eq(&path) {
            if ie.object_hash() == object.hash().map_err_with("could not hash object when comparing hashes")? {
                status.push_str(format!("").bright_green().as_ref())
            }
        }
    }

    Ok(status)
}

pub fn search_dir(path: PathBuf) -> Result<Vec<ObjectData>> {
    if path.is_dir() {
        let mut objects = Vec::new();
        for e in std::fs::read_dir(path)? {
            objects.extend(search_dir(e?.path())?)
        }
        Ok(objects)
    } else {
        let file = std::fs::File::open(&path)?;
        let object = Object::try_from(file)?;
        Ok(vec![(path, object)])
    }
}
