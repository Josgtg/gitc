use std::collections::HashSet;
use std::ffi::OsStr;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use colored::Colorize;

use crate::byteable::Byteable;
use crate::error::CustomResult;
use crate::hashing::Hash;
use crate::index::IndexEntry;
use crate::object::Object;
use crate::{fs, gitignore, Constants, Result};

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
    let all_paths = fs::path::read_dir_paths(&root_path)
        .map_err_with("could not read root directory entries")?;

    let files = gitignore::not_in_gitignore(&root_path, all_paths)?;

    // Getting files from working tree
    let mut objects = Vec::new();
    for p in files.into_iter() {
        objects.extend(search_dir(p).map_err_with("could not get working tree objects")?);
    }

    let mut changes_staged = String::from("Changes to be commited:\n");
    let mut include_staged = false;
    let mut changes_not_staged = String::from("Untracked files:\n");
    let mut include_not_staged = false;

    // Checking differences
    let mut object_hash: Hash;
    for (path, object) in objects {
        if paths_set.contains(path.as_os_str()) {
            object_hash = Hash::new(
                object
                    .as_bytes()
                    .map_err_with("could not encode object")?
                    .as_ref(),
            );
            if hashes_set.contains(&object_hash) {
                changes_staged
                    .push_str(format!("\tmodified: {}\n", path.to_string_lossy()).as_ref());
                include_staged = true;
            } else {
                changes_not_staged
                    .push_str(format!("\tmodified: {}\n", path.to_string_lossy()).as_ref());
                include_not_staged = true;
            };
        } else {
            changes_not_staged.push_str(format!("\t{}\n", path.to_string_lossy()).as_ref());
            include_not_staged = true;
        }
    }
    changes_staged.pop();
    changes_not_staged.pop();

    if !include_staged && !include_not_staged {
        return Ok("working tree clean, nothing to commit".into());
    }

    let mut status = String::new();
    if include_staged {
        status.push_str(format!("{}", changes_staged.green()).as_ref());
    }
    if include_not_staged {
        status.push_str(format!("\n{}", changes_not_staged.red()).as_ref());
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
        let mut file = std::fs::File::open(&path).map_err_with("could not open file")?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err_with("could not read file")?;
        let mut cursor = Cursor::new(data);

        let object = Object::from_bytes(&mut cursor).map_err_with("could not decode object")?;

        Ok(vec![(path, object)])
    }
}
