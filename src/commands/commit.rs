use anyhow::{Context, Result};

use crate::fs;
use crate::fs::index::read_index_file;
use crate::fs::object::write_object;
use crate::object::Object;
use crate::object::commit::{CommitUser, CommitUserKind};
use crate::object::tree::TreeBuilder;

/// Creates a commit object file, a tree from the current index contents and updates the branch
/// HEAD points to to point at the new commit.
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

    let mut parents = Vec::new();
    let previous_commit = fs::get_last_commit_hash().context("could not get last commit hash")?;
    if let Some(h) = previous_commit {
        parents.push(h);
    }

    let commit = Object::Commit {
        tree,
        parents: parents.into(),
        author: CommitUser::default(CommitUserKind::Author),
        committer: CommitUser::default(CommitUserKind::Committer),
        message: message.into(),
    };

    let commit_hash = write_object(&commit).context("could not write commit file")?;

    let current_branch =
        fs::get_current_branch_path().context("could not get current branch path")?;
    std::fs::write(current_branch, commit_hash.to_string().as_bytes())
        .context("could not update current branch (make it point to the new commit))")?;

    Ok("Commited changes successfully\n".into())
}
