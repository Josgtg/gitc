use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use colored::Colorize;

use crate::Constants;
use crate::fs;

struct FileStatus {
    path: PathBuf,
    status: Status,
    tracked: bool,
}

#[allow(unused)]
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
pub fn status() -> Result<String> {
    let mut status = Vec::new();

    let index = fs::index::read_index_file().context("could not read index file")?;

    let hashes_in_index = 0;

    let all_files = fs::read_not_ignored_paths(&Constants::working_tree_root_path()).context("could not get files in working tree")?;

    let working_tree = fs::object::as_blob_objects(all_files).context("could not read files in working tree as objects")?;

    for o in working_tree {

    }

    Ok(format_status(status))
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
            Status::Unchanged => continue,
        };
        if s.tracked {
            tracked_files = true;
            tracked.push_str(&status_str);
        } else {
            untracked_files = true;
            untracked.push_str(&status_str);
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
