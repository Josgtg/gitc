use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::CustomResult;
use crate::fs;
use crate::gitignore;
use crate::hashing::Hash;
use crate::index::{IndexEntry, builder::IndexBuilder};
use crate::object::Object;
use crate::{Constants, Result};

const PATTERN_EVERY_FILE: &str = ".";

type ObjectData = (PathBuf, Hash);

pub fn add(files: &[OsString]) -> Result<String> {
    if files.is_empty() {
        return Ok("There were no files to add".into());
    }

    let root_path = Constants::repository_folder_path();

    let paths: Vec<PathBuf> = if files
        // checking for "add all" pattern
        .first()
        .expect("file did not have first element despite being checked for emptiness")
        == PATTERN_EVERY_FILE
    {
        fs::path::read_dir_paths(&root_path)
            .map_err_with("could not read paths in repository folder")?
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    // Discarding ignored files, important to check as relative path
    let filtered_paths: Vec<PathBuf> = gitignore::not_in_gitignore(&root_path, paths).map_err_with("could not read get filtered files from .gitignore")?;

    // reading all not ignored files as blob objects
    let mut objects = Vec::new();
    for p in filtered_paths {
        objects.extend(add_dir(p).map_err_with("failed to add dir")?);
    }

    // ordering entries in lexicographical order
    objects.sort_by(|(p1, _), (p2, _)| PathBuf::cmp(p1, p2));

    // getting previous index to update it
    let previous_index = fs::index::read_index_file().map_err_with("could not read index file")?;
    let mut index_builder = IndexBuilder::from(previous_index);

    // building a set containing hashes already in index to avoid adding a file twice
    let mut hashes_already_in_index: HashSet<Hash> = HashSet::new();
    for h in index_builder.iter_index_entries().map(|o| o.object_hash()) {
        hashes_already_in_index.insert(h);
    }

    // adding index entries
    let mut index_entry: IndexEntry;
    for (p, o) in objects {
        if hashes_already_in_index.contains(&o) {
            // file is already in index
            continue;
        }
        index_entry = IndexEntry::try_from_file(&p, o)
            .map_err_with(format!("could not create index entry from file: {:?}", p))?;
        index_builder.add_index_entry(index_entry);
    }

    let index = index_builder.build();

    fs::index::write_index_file(index).map_err_with("could not write to index file")?;

    Ok("Added files successfully".into())
}

/// This function calls itself recursively for every subdirectory inside of `dir`, until there are
/// no more subdirectories, calling `add_file` for every file inside `dir`.
fn add_dir(path: PathBuf) -> Result<Vec<ObjectData>> {
    let err_message = format!("could not add file when returning from add_dir: {path:?}");
    if !path.is_dir() {
        // is a file
        return Ok(vec![add_file(path).map_err_with(err_message)?]);
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
    let file = std::fs::File::open(&path).map_err_with(format!(
        "could not open path when trying to add file: {path:?}"
    ))?;

    let object = Object::try_from(file)
        .map_err_with(format!("could not create object from file: {path:?}").as_str())?;
    let hash = fs::object::write_to_object_dir(object).map_err_with(
        format!("could not get file hash because writing to object dir failed when trying to add file: {path:?}"),
    )?;

    Ok((path, hash))
}
