use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::fs;
use crate::hashing::Hash;
use crate::object::Object;

use super::TreeEntry;

/// Given a series of tree entries, searches trough it and any subtrees it may have and returns
/// all the entries included in the tree structure.
///
/// It's important to know that all the paths in the returned tree entry will have their path
/// relative to the root tree.
pub fn get_all_tree_entries(entries: Vec<TreeEntry>) -> Result<Vec<TreeEntry>> {
    let mut paths = Vec::with_capacity(entries.len());
    for e in entries {
        if e.path.is_dir() {
            paths.extend(
                get_subtree(&e.path, e.hash.clone()).context("could not get subtree paths")?,
            );
        } else {
            paths.push(e);
        }
    }
    Ok(paths)
}

/// Reads the tree with the provided hash and goes trough all it's entries, calling itself
/// recursively if the subtree has another subtree on it.
fn get_subtree(path: &Path, hash: Hash) -> Result<Vec<TreeEntry>> {
    let tree = fs::object::read_object(hash).context("could not read tree")?;
    let entries = match tree {
        Object::Tree { entries } => entries,
        _ => bail!("expected a tree object"),
    };

    let mut paths = Vec::with_capacity(entries.len());

    let mut whole_path: PathBuf;
    for mut e in entries {
        whole_path = path.join(&e.path);
        if whole_path.is_dir() {
            paths.extend(get_subtree(&whole_path, e.hash).context("could not get subtree paths")?);
        } else {
            e.path = whole_path;
            paths.push(e);
        }
    }

    Ok(paths)
}
