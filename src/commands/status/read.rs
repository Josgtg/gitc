use std::path::PathBuf;
use std::collections::HashMap;

use anyhow::{bail, Result, Context};

use crate::Constants;
use crate::utils;
use crate::fs;
use crate::object::{self, Object};
use crate::object::tree::TreeEntry;
use crate::index::IndexEntryCache;
use crate::hashing::Hash;

use super::status::FileData;

pub fn read_working_tree_data() -> Result<Vec<FileData>> {
    let root_dir = Constants::working_tree_root_path();
    let working_tree = fs::path::get_all_files_bufered(&root_dir)
        .context("could not get files in working tree")?;

    let working_tree_data = working_tree
        .into_iter()
        .map(|file| FileData {
            // important to convert the paths to relative paths
            path: utils::path::relative_path(&file.path, &root_dir).unwrap_or(file.path),
            reader: file.reader,
            cache: file.cache,
        })
        .collect();

    Ok(working_tree_data)
}


pub type CommitData = HashMap<PathBuf, Hash>;
/// Returns a Map with the hash of every file in the previous commit mapped to its path.
///
/// # Returns
///
/// A result of an option, this option will be `None` only if there is no previous commit.
///
/// # Errors
///
/// This function can fail if the existence of a previous commit could not be verified.
pub fn read_commit_data() -> Result<Option<CommitData>> {
    // checking if there is a previous commit, otherwise we just leave the data related to the
    // commit
    let previous_commit_hash =
        fs::get_last_commit_hash().context("could not get last commit hash")?;

    if previous_commit_hash.is_none() {
        log::info!("no previous commit found");
        return Ok(None);
    }

    let mut commit_data = HashMap::new();

    let commit = fs::object::read_object(previous_commit_hash.expect("should never be None"))
        .context("could not read last commit")?;

    let tree_obj: Object = if let Object::Commit { tree, .. } = commit {
        fs::object::read_object(tree).context("could not read commit tree")?
    } else {
        bail!("expected tree")
    };

    let all_entries: Vec<TreeEntry> = if let Object::Tree { entries } = tree_obj {
        object::tree::get_all_tree_entries(entries).context("could not get all tree entries")?
    } else {
        bail!("expected commit")
    };

    for e in all_entries {
        commit_data.insert(e.path, e.hash);
    }

    Ok(Some(commit_data))
}


pub type IndexData = HashMap<PathBuf, (Hash, IndexEntryCache)>;
/// Returns a Map with the hash and cache of every file in the current index mapped to its path.
///
/// # Errors
///
/// This function can fail if the index file could not be read.
pub fn read_index_data() -> Result<IndexData> {
    let mut index_data = IndexData::default();

    let index = fs::index::read_index_file().context("could not read index file")?;

    let mut hash: Hash;
    let mut cache: IndexEntryCache;
    let mut path: PathBuf;
    for mut ie in index.into_entries() {
        hash = ie.object_hash();
        cache = std::mem::take(&mut ie.cache_data);
        path = ie.into_path();

        index_data.insert(path, (hash, cache));
    }

    Ok(index_data)
}

