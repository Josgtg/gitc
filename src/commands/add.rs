use std::ffi::OsString;
use std::path::PathBuf;

use crate::fs;
use crate::hashing::Hash;
use crate::index::{IndexBuilder, IndexEntry};
use crate::object::Object;
use crate::{Constants, Result};

const PATTERN_EVERY_FILE: &'static str = ".";

type ObjectData = (PathBuf, Hash);

pub fn add(files: &[OsString]) -> Result<String> {
    if files.is_empty() {
        return Ok("There were no files to add".into());
    }

    let folder_path = Constants::repository_folder_path();

    let files_to_ignore = fs::path::read_gitignore(&folder_path)?;

    let paths: Vec<PathBuf>;
    if files
        .first()
        .expect("file did not have first element despite being checked for emptiness")
        == PATTERN_EVERY_FILE
    {
        // pattern to add every file
        paths = fs::path::read_dir_paths(&folder_path)?;
    } else {
        paths = files.iter().map(|p| PathBuf::from(p)).collect();
    }

    // Discarding ignored files
    let filtered_paths: Vec<PathBuf> = paths
        .into_iter()
        .map(|p| fs::path::relative_path(&p, &folder_path).unwrap_or(p))
        .filter(|p| !files_to_ignore.contains(p))
        .collect();

    let mut objects = Vec::new();
    for p in filtered_paths {
        objects.extend(add_dir(p).expect("failed to add dir"));
    }

    // updating index with the new files
    let previous_index = fs::index::read_index_file().expect("could not read index file");
    let mut index_builder = IndexBuilder::from(previous_index);
    let mut index_entry: IndexEntry;
    for o in objects {
        index_entry = IndexEntry::try_from_file(&o.0, o.1)
            .expect(format!("could not create index entry from file: {:?}", o.0).as_str());
        index_builder.add_index_entry(index_entry);
    }
    let index = index_builder.build();

    fs::index::write_index_file(index).expect("could not write to index");

    Ok("Added files successfully".into())
}

/// This function calls itself recursively for every subdirectory inside of `dir`, until there are
/// no more subdirectories, calling `add_file` for every file inside `dir`.
fn add_dir(dir: PathBuf) -> Result<Vec<ObjectData>> {
    let mut objects = Vec::new();
    for p in fs::path::read_dir_paths(&dir)? {
        if p.is_dir() {
            objects.extend(add_dir(p)?);
        } else {
            objects.push(add_file(p)?);
        }
    }
    Ok(objects)
}

// Writes a file to the object dir, returning the object hash and the path of the file the object
// represents.
//
// # Errors
//
// This function can fail if:
// - The file in `path` couldn't be opened.
// - It wasn't possible to create an Object from the file.
// - It wasn't possible to write to the object dir.
fn add_file(path: PathBuf) -> Result<ObjectData> {
    let file = std::fs::File::open(&path)?;

    let object = Object::try_from(file).expect(format!("could not create object from file: {path:?}").as_str());
    let hash = fs::object::write_to_object_dir(object)?;

    Ok((path, hash))
}
