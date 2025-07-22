use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::hashing::Hash;
use crate::object::Object;
use crate::Constants;
use crate::{fs, object};

struct FileWithStatus {
    path: PathBuf,
    status: Status,
    stage_status: StageStatus
}

struct FileData {
    path: PathBuf,
    hash: Hash,
}

#[allow(unused)]
#[derive(Eq, PartialEq, Debug)]
enum Status {
    New,
    Modified,
    Deleted,
    Moved { previous: PathBuf },
    Unchanged,
}

#[derive(Eq, PartialEq, Debug)]
enum StageStatus {
    Commit,
    NotCommit,
    Untracked,
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
    let all_files = fs::read_not_ignored_paths(&Constants::working_tree_root_path())
        .context("could not get files in working tree")?;

    let working_tree = fs::object::as_blob_objects(all_files)
        .context("could not read files in working tree as objects")?;
    // Storing the information of the files in the working tree
    let mut working_tree_data = Vec::new();
    for o in working_tree {
        working_tree_data.push(FileData {
            hash: o.blob.hash().context("could not hash object")?,
            path: o.path,
        });
    }

    // used to determine if a file is being tracked
    let index = fs::index::read_index_file().context("could not read index file")?;
    // this set is useful to determine if a file is being tracked or not
    let mut index_paths: HashSet<PathBuf> =
        HashSet::from_iter(index.entries().map(|ie| ie.path().to_path_buf()));
    let index_hashes: HashSet<Hash> = 
        HashSet::from_iter(index.entries().map(|ie| ie.object_hash()));

    // This two sets contain the information we will compare the files with
    let mut commit_data = Vec::new();
    // Where to get the information to compare the working tree with?
    let previous_commit_hash =
        fs::get_last_commit_hash().context("could not get last commit hash")?;
    if let Some(hash) = previous_commit_hash {
        // if there is a previous commit, we use that
        let commit = fs::object::read_object(hash).context("could not read last commit")?;
        if let Object::Commit { tree, .. } = commit {
            let tree_obj = fs::object::read_object(tree).context("could not read commit tree")?;
            if let Object::Tree { entries } = tree_obj {
                let all_entries = object::tree::get_all_tree_entries(entries)
                    .context("could not get all tree entries")?;
                for e in all_entries {
                    commit_data.push(FileData {
                        hash: e.hash,
                        path: e.path,
                    });
                }
            } else {
                bail!("expected tree")
            }
        } else {
            bail!("expected commit")
        }
    } else {
        // otherwise we get it from the index
        for e in index.into_entries() {
            commit_data.push(FileData {
                hash: e.object_hash(),
                path: e.into_path(),
            });
        }
    };

    let commit_paths: HashSet<&PathBuf> = HashSet::from_iter(commit_data.iter().map(|fd| &fd.path));
    let commit_hashes: HashSet<&Hash> = HashSet::from_iter(commit_data.iter().map(|fd| &fd.hash));

    // we will be removing paths from this sets, the remaining ones would be the deleted files
    let mut commit_deleted: HashSet<PathBuf> =
        HashSet::from_iter(commit_data.iter().map(|fd| fd.path.to_path_buf()));

    let mut files_statuses = Vec::new();
    let mut status: Status;
    let mut stage_status: StageStatus;
    for FileData { path, hash } in working_tree_data {
        stage_status = if !index_paths.contains(&path) {
            // If the path of a file is not present in index, it is being untracked
            StageStatus::Untracked
        } else {
            if index_hashes.contains(&hash) {
                // file and hash present, the file is staged for commit in it's current state
                StageStatus::Commit
            } else {
                // file present but different data, then the file is tracked but there are unstaged changes
                StageStatus::NotCommit
            }
        };
        
        index_paths.remove(&path);
        commit_deleted.remove(&path);

        status = if commit_paths.contains(&path) {
            if commit_hashes.contains(&hash) {
                // same path, same data
                Status::Unchanged
            } else {
                // same path, new data
                Status::Modified
            }
        } else {
            if commit_hashes.contains(&hash) {
                // new path, same data
                let previous = commit_data
                    .iter()
                    .filter(|fd| fd.hash == hash)
                    .next()
                    .context("could not find hash in commit files, despite this branch meaning it exists inside commit files")?
                    .path.clone();
                Status::Moved { previous }
            } else {
                // new path, new data
                Status::New
            }
        };

        files_statuses.push(FileWithStatus {
            path,
            status,
            stage_status,
        });
    }

    // file is deleted and not in index, it is being tracked
    for path in commit_deleted {
        files_statuses.push(FileWithStatus {
            path,
            status: Status::Deleted,
            stage_status: StageStatus::Commit,
        });
    }

    // file is deleted but still in index, then it is not staged for commit.
    for path in index_paths {
        files_statuses.push(FileWithStatus {
            path,
            status: Status::Deleted,
            stage_status: StageStatus::NotCommit,
        });
    }

    Ok(format_status(files_statuses))
}

/// Given a list of file statuses, returns a formatted string depicting this status for every file.
fn format_status(status: Vec<FileWithStatus>) -> String {
    let filtered_status: Vec<FileWithStatus> = status.into_iter().filter(|fws| fws.status != Status::Unchanged).collect();

    let mut header = format!(
        "On branch {}",
        fs::get_current_branch_name().unwrap_or(Constants::DEFAULT_BRANCH_NAME.into())
    );

    if filtered_status.is_empty() {
        return format!("{}\nThere is nothing to commit, all clean!\n", header)
    }

    let mut commit = String::new();
    let mut notcommit = String::new();
    let mut untracked = String::new();
    let mut status_str: String;
    let mut path_str: String;
    for s in filtered_status {
        path_str = s.path.to_string_lossy().to_string();
        status_str = match &s.status {
            Status::New => format!("\tnew file:\t{}\n", path_str),
            Status::Moved { previous } => format!("\tmoved:\t{} -> {}\n", previous.to_string_lossy().to_string(), path_str),
            Status::Deleted => format!("\tdeleted:\t{}\n", path_str),
            Status::Modified => format!("\tmodified:\t{}\n", path_str),
            Status::Unchanged => continue,
        };
        match s.stage_status {
            StageStatus::Commit => commit.push_str(&status_str),
            StageStatus::NotCommit => notcommit.push_str(&status_str),
            StageStatus::Untracked => untracked.push_str(&status_str),
        };
    }

    if !commit.is_empty() {
        header = format!("{}\nChanges staged for commit:\n{}", header, commit.green());
    }
    if !notcommit.is_empty() {
        header = format!("{}\nNot staged for commit:\n{}", header, notcommit.red());
    }
    if !untracked.is_empty() {
        header = format!("{}\nUntracked files:\n{}", header, untracked.red());
    }

    header
}
