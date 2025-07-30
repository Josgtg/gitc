use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result};

use crate::byteable::Byteable;
use crate::fs;
use crate::hashing::Hash;
use crate::index::{builder::IndexBuilder, IndexEntry};
use crate::Constants;

const PATTERN_EVERY_FILE: &str = ".";

/// Fetches all files from the worktree (not in .gitignore), creates blob objects for all of them,
/// creates index entries from those objects and adds them to the index file.
pub fn add(files: &[OsString]) -> Result<String> {
    let root_path = Constants::working_tree_root_path();

    let filtered_paths: Vec<PathBuf> = if files[0] == PATTERN_EVERY_FILE {
        fs::read_not_ignored_paths(&root_path).context("could not filter ignored files")?
    } else {
        // We do not check if a file is in .gitignore if it's deliberately added
        files.iter().map(PathBuf::from).collect()
    };

    if filtered_paths.is_empty() {
        return Ok("There were no files to add\n".into())
    }

    // reading all (not ignored) files as blob objects
    let objects = fs::object::as_blob_objects(filtered_paths).context("could not get objects")?;

    // getting previous index to update it
    let previous_index = fs::index::read_index_file().context("could not read index file")?;

    // building a set containing hashes already in index to avoid adding a file twice
    let mut hashes_already_in_index: HashSet<Hash> = HashSet::new();
    let mut paths_already_in_index: HashSet<PathBuf> = HashSet::new();
    for ie in previous_index.entries() {
        hashes_already_in_index.insert(ie.object_hash());
        paths_already_in_index.insert(ie.path().to_owned());
    }

    // adding index entries
    let mut index_builder = IndexBuilder::from(previous_index);
    let mut index_entry: IndexEntry;
    let mut path: PathBuf;
    let mut hash: Hash;
    let mut bytes: Rc<[u8]>;
    for o in objects {
        path = o.path;
        bytes = o
            .blob
            .as_bytes()
            .context(format!("could not encode object for file: {:?}", path))?;

        hash = Hash::compute(bytes.as_ref());

        if paths_already_in_index.contains(&path) {
            // file already in index, we delete it since we won't be needing it and it would be
            // useful later (paths left at the end are deleted files)
            paths_already_in_index.remove(&path);
            if !hashes_already_in_index.contains(&hash) {
                // ...and has been modified. We remove it and add it as if it was a new file
                index_builder.remove_index_entry_by_path(&path);
            } else {
                // ...and is unchanged, we just ignore it
                continue;
            }
        }
        // if the `if` above is not triggered, this is a new file

        index_entry = IndexEntry::try_from_file(&path, hash.clone()).context(format!(
            "could not create index entry from file: {:?}",
            &path
        ))?;

        fs::object::write_to_object_dir(&bytes, &hash).context("could not write to object dir")?;
        index_builder.add_index_entry(index_entry);
    }

    for p in paths_already_in_index {
        index_builder.remove_index_entry_by_path(&p);
    }

    let index = index_builder.build();

    fs::index::write_index_file(index).context("could not write to index file")?;

    Ok("Added files successfully\n".into())
}
