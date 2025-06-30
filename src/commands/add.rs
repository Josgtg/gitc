use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::CustomResult;
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

    let files_to_ignore =
        fs::path::read_gitignore(&folder_path).map_err_with("could not read .gitignore file")?;

    let paths: Vec<PathBuf>;
    // checking for "add all" pattern
    if files
        .first()
        .expect("file did not have first element despite being checked for emptiness")
        == PATTERN_EVERY_FILE
    {
        paths = fs::path::read_dir_paths(&folder_path)
            .map_err_with("could not read paths in repository folder")?;
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
        objects.extend(add_dir(p).map_err_with("failed to add dir")?);
    }

    // updating index with the new files
    let previous_index = fs::index::read_index_file().map_err_with("could not read index file")?;
    let mut index_builder = IndexBuilder::from(previous_index);
    let mut index_entry: IndexEntry;
    for o in objects {
        index_entry = IndexEntry::try_from_file(&o.0, o.1)
            .map_err_with(format!("could not create index entry from file: {:?}", o.0))?;
        index_builder.add_index_entry(index_entry);
    }
    let index = index_builder.build();

    fs::index::write_index_file(index).map_err_with("could not write to index")?;

    Ok("Added files successfully".into())
}

/// This function calls itself recursively for every subdirectory inside of `dir`, until there are
/// no more subdirectories, calling `add_file` for every file inside `dir`.
fn add_dir(path: PathBuf) -> Result<Vec<ObjectData>> {
    let err_message = format!("could not add file when returning from add_dir: {path:?}");
    if !path.is_dir() {
        // is a file
        return Ok(vec![
            add_file(path).map_err_with(err_message)?
        ]);
    }

    let mut objects = Vec::new();
    for p in fs::path::read_dir_paths(&path)? {
        objects.extend(add_dir(p)?);
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
    let file =
        std::fs::File::open(&path).map_err_with(format!("could not open path when trying to add file: {path:?}"))?;

    let object = Object::try_from(file)
        .map_err_with(format!("could not create object from file: {path:?}").as_str())?;
    let hash = fs::object::write_to_object_dir(object).map_err_with(
        format!("could not get file hash because writing to object dir failed when trying to add file: {path:?}"),
    )?;

    Ok((path, hash))
}
