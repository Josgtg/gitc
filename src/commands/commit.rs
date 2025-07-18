use std::str::FromStr;

use anyhow::{Context, Result};

use crate::fs::index::read_index_file;
use crate::fs::object::write_object;
use crate::hashing::Hash;
use crate::object::Object;
use crate::object::commit::CommitUser;
use crate::object::tree::TreeBuilder;

#[allow(unused)]
pub fn commit(message: &str) -> Result<String> {
    // Creating a tree from every file in the index
    let mut tree_builder = TreeBuilder::new();
    let index = read_index_file().context("could not read index file")?;

    for e in index.entries() {
        tree_builder.add_object(e.mode, e.path().to_owned(), e.object_hash());
    }

    let tree = tree_builder
        .build_and_write()
        .context("could not write tree object")?;

    // Getting the direct parent
    let current_branch =
        crate::fs::get_current_branch_path().context("could not get current branch")?;
    let mut parents = Vec::new();
    if current_branch.exists() {
        // If it does not exist then this is the first commit and there is no parents
        let parent_hash_bytes =
            std::fs::read(&current_branch).context("could not read current branch reference")?;
        let parent_hash_str = String::from_utf8_lossy(&parent_hash_bytes);
        parents.push(
            Hash::from_str(&parent_hash_str)
                .context("could not get hash from contents on current branch")?,
        );
    }

    let commit = Object::Commit {
        tree,
        parents: parents.into(),
        author: CommitUser::default(),
        committer: CommitUser::default(),
        message: message.into(),
    };

    let commit_hash = write_object(&commit).context("could not write commit file")?;

    std::fs::write(current_branch, commit_hash.to_string().as_bytes())
        .context("could not update current branch (make it point to the new commit))")?;

    Ok("Commited changes successfully".into())
}
