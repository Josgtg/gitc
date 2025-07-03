use std::collections::HashSet;
use std::ffi::OsStr;
use std::path::PathBuf;

use colored::Colorize;

use crate::error::CustomResult;
use crate::hashing::Hash;
use crate::index::IndexEntry;
use crate::object::Object;
use crate::{fs, Constants, Result};

use crate::fs::index::read_index_file;

type ObjectData = (PathBuf, Object);

/// Returns a string with the status of the repository. It lists:
/// - The changes respective to the last commit.
/// - The changes respective to the working tree.
///
/// # Errors
///
/// This function can fail if:
/// - The index file couldn't be read.
/// - Could not get object data from a file in the working tree.
pub fn status() -> Result<String> {
    // Getting index data an placing it in hash sets for easy access
    let index = read_index_file().map_err_with("could not read from index file")?;
    let paths_set: HashSet<&OsStr> = HashSet::from_iter(index.entries().map(IndexEntry::path));
    let hashes_set: HashSet<Hash> =
        HashSet::from_iter(index.entries().map(IndexEntry::object_hash));

    // Getting filtered files
    let root_path = Constants::repository_folder_path();
    let files = fs::path::not_in_gitignore(
        fs::path::read_dir_paths(&root_path)
            .map_err_with("could not read root directory entries")?
    )?;

    // Getting files from working tree
    let mut objects = Vec::new();
    for p in files.into_iter() {
        objects.extend(
            search_dir(p).map_err_with("could not get working tree objects")?
        );
    }

    let mut changes_staged = String::from("Changes staged for commit:\n");
    let mut include_staged = false;
    let mut changes_not_staged = String::from("Changes not staged for commit:\n");
    let mut include_not_staged = false;

    // Checking differences
    for (path, object) in objects {
        if paths_set.contains(path.as_os_str()) {
            if hashes_set.contains(
                &object
                    .hash()
                    .map_err_with("could not get object hash to compare with index entry")?,
            ) {
                changes_staged
                    .push_str(format!("\tmodified: {}\n", path.to_string_lossy()).as_ref());
                include_staged = true;
            } else {
                changes_not_staged
                    .push_str(format!("\tmodified: {}\n", path.to_string_lossy()).as_ref());
                include_not_staged = true;
            };
        } else {
            changes_not_staged
                .push_str(format!("\tnew file: {}\n", path.to_string_lossy()).as_ref());
            include_not_staged = true;
        }
    }
    changes_staged.pop();
    changes_not_staged.pop();

    if !include_staged && ! include_not_staged {
        return Ok("working tree clean, nothing to commit".into())
    }

    let mut status = String::new();
    if include_staged {
        status.push_str(format!("{}", changes_staged.bright_green()).as_ref());
    }
    if include_not_staged {
        status.push_str(format!("\n\n{}", changes_not_staged.bright_red()).as_ref());
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
