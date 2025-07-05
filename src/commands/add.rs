use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Context;
use anyhow::Result;

use crate::Constants;
use crate::byteable::Byteable;
use crate::fs;
use crate::gitignore;
use crate::hashing::Hash;
use crate::index::{IndexEntry, builder::IndexBuilder};

const PATTERN_EVERY_FILE: &str = ".";

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
        fs::path::read_dir_paths(&root_path).context("could not read paths in repository folder")?
    } else {
        files.iter().map(PathBuf::from).collect()
    };

    // Discarding ignored files, important to check as relative path
    let filtered_paths: Vec<PathBuf> = gitignore::not_in_gitignore(&root_path, paths)
        .context("could not read get filtered files from .gitignore")?;

    // reading all (not ignored) files as blob objects
    let mut objects = fs::object::as_objects(filtered_paths).context("could not get objecs")?;

    // ordering entries in lexicographical order
    objects.sort_by(|o1, o2| PathBuf::cmp(&o1.path, &o2.path));

    // getting previous index to update it
    let previous_index = fs::index::read_index_file().context("could not read index file")?;
    let mut index_builder = IndexBuilder::from(previous_index);

    // building a set containing hashes already in index to avoid adding a file twice
    let mut hashes_already_in_index: HashSet<Hash> = HashSet::new();
    for h in index_builder.iter_index_entries().map(|o| o.object_hash()) {
        hashes_already_in_index.insert(h);
    }

    // adding index entries
    let mut index_entry: IndexEntry;
    let mut path: PathBuf;
    let mut hash: Hash;
    let mut bytes: Rc<[u8]>;
    for o in objects {
        path = o.path;
        bytes = o
            .object
            .as_bytes()
            .context(format!("could not encode object for file: {:?}", path))?;
        hash = Hash::new(bytes.as_ref());
        if hashes_already_in_index.contains(&hash) {
            // file is already in index
            continue;
        }
        index_entry = IndexEntry::try_from_file(&path, hash.clone()).context(format!(
            "could not create index entry from file: {:?}",
            &path
        ))?;
        fs::object::write_to_object_dir(&bytes, &hash).context("could not write to object dir")?;
        index_builder.add_index_entry(index_entry);
    }

    let index = index_builder.build();

    fs::index::write_index_file(index).context("could not write to index file")?;

    Ok("Added files successfully".into())
}
