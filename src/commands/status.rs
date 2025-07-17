use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use colored::Colorize;

use crate::Constants;
use crate::fs;
use crate::hashing::Hash;
use crate::index::IndexEntry;

use crate::fs::index::read_index_file;

struct FileStatus {
    path: PathBuf,
    status: Status,
    tracked: bool,
}

enum Status {
    New,
    Modified,
    Deleted,
    Moved { previous: PathBuf },
    Unchanged,
}

/// Returns a string with the status of the repository. It lists:
/// - The changes respective to the last commit.
/// - The changes respective to the working tree.
///
/// # Errors
///
/// This function can fail if:
/// - The index file couldn't be read.
/// - Could not get object data from a file in the working tree.
#[allow(unused)]
pub fn status() -> Result<String> {
    // Getting index data an placing it in hash sets for easy access
    let index = read_index_file().context("could not read from index file")?;
    let paths_set: HashSet<&Path> = HashSet::from_iter(index.entries().map(IndexEntry::path));
    let hashes_set: HashSet<Hash> =
        HashSet::from_iter(index.entries().map(IndexEntry::object_hash));

    let root_path = Constants::repository_folder_path();

    let paths = fs::path::read_not_ignored_paths(&root_path)
        .context("could not get files from directory")?;

    let objects = fs::object::as_blob_objects(paths).context("could not create objects")?;

    // Getting files from working tree

    let mut changes_staged = String::from("Changes to be commited:\n");
    let mut include_staged = false;
    let mut changes_not_staged = String::from("Untracked files:\n");
    let mut include_not_staged = false;

    // Checking differences
    let mut status = Vec::with_capacity(objects.len());
    let mut path: PathBuf;
    let mut hash: Hash;
    for o in objects {
        // Implement once commit is done
    }

    let s = format_status(status);

    Ok(s)
}

/// Given a list of file statuses, returns a formatted string depicting this status for every file.
fn format_status(status: Vec<FileStatus>) -> String {
    let mut tracked = String::from("Changes to commit:\n");
    let mut tracked_files = false;
    let mut untracked = String::from("Untracked files:\n");
    let mut untracked_files = false;

    let mut status_str: String;
    for s in status {
        status_str = match &s.status {
            Status::New => format!("new file: {:?}\n", s.path),
            Status::Moved { previous } => format!("moved: {:?} -> {:?}\n", previous, s.path),
            Status::Deleted => format!("deleted: {:?}\n", s.path),
            Status::Modified => format!("modified: {:?}\n", s.path),
            Status::Unchanged => String::new(),
        };
        if let Status::Unchanged = s.status {
            continue;
        }
        match s.tracked {
            true => {
                tracked_files = true;
                tracked.push_str(&status_str);
            }
            false => {
                untracked_files = true;
                untracked.push_str(&status_str);
            }
        }
    }

    let mut final_str = String::new();

    if tracked_files {
        final_str.push_str(&tracked.green());
    }
    if untracked_files {
        final_str.push('\n');
        final_str.push_str(&untracked.red());
    }

    final_str
}
