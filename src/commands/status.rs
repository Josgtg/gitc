use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::error::WarnUnwrap;
use crate::fs;
use crate::hashing::Hash;
use crate::index::IndexEntryCache;
use crate::object;
use crate::object::tree::TreeEntry;
use crate::object::Object;
use crate::Constants;

struct FileWithStatus {
    path: PathBuf,
    status: Status,
    stage_status: StageStatus,
}

struct FileData {
    path: PathBuf,
    data: Rc<[u8]>,
    cache: IndexEntryCache,
}

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
    let commit_data = read_commit_data().context("could not get commit data")?;
    let index_data = read_index_data().context("could not get index data")?;
    let working_tree_data = read_working_tree_data().context("could not get working tree data")?;

    let file_statuses = determine_statuses(commit_data, index_data, working_tree_data);

    Ok(format_status(file_statuses))
}

#[allow(unused_assignments)]
fn determine_statuses(
    mut commit_data: CommitData,
    mut index_data: IndexData,
    working_tree_data: Vec<FileData>,
) -> Vec<FileWithStatus> {
    let mut file_statuses = Vec::new();

    // Used to determine at the end if a file was new or it was the new name of a previous file
    // (has the same hash as a deleted file).
    let mut possibly_moved_files: HashMap<Hash, (PathBuf, StageStatus)> = HashMap::new();

    let mut status: Status;
    let mut stage_status: StageStatus;

    let mut hash: Hash = Hash::default();
    // Used to avoid hashing twice in the same iteration
    let mut hash_updated: bool;

    // Contains the data stored in the index and commit for the current iteration's file
    let mut index_specific_data = HashCachePair::default();

    for FileData { path, data, cache } in working_tree_data {
        hash_updated = false;

        // Notice the use of the `remove` function here instead of the `get` one, this way we can
        // keep track of the files that appear on the index but not in the working tree, since the
        // files left at the end of the loop are only in the index. Same with `commit_data::remove`
        // below.
        stage_status = if let Some(isd) = index_data.remove(&path) {
            index_specific_data = isd;
            // Path in index
            if index_specific_data.cache.matches_loose(&cache) {
                // And metadata shows it's unchanged; we have the current version of the file
                // in the index so it is staged for commit
                StageStatus::Commit
            } else {
                // If checking with cache is not successful, we hash the file data and run the
                // checks with the hash
                if !hash_updated {
                    hash = Hash::compute(&data);
                    hash_updated = true;
                }
                if index_specific_data.hash == hash {
                    // Index contains path and hash, so this file is being tracked in it's current
                    // state
                    StageStatus::Commit
                } else {
                    // The path exists in the index, but there is different data, so there are
                    // unstaged changes
                    StageStatus::NotCommit
                }
            }
        } else {
            // If the path of the file is not in the index, then it is just being untracked
            StageStatus::Untracked
        };

        status = if stage_status != StageStatus::Untracked {
            // Paths here appear on index
            if let Some(commit_specific_hash) = commit_data.remove(&path) {
                // Path appears on index and previous commit
                if commit_specific_hash == index_specific_data.hash {
                    // Same path and hash in both index and previous commit, the file is unchanged
                    Status::Unchanged
                } else {
                    // Different data but same path, the file has been modified
                    Status::Modified
                }
            } else {
                // File in in index but not in previous commit, so this file is new
                if !hash_updated {
                    hash = Hash::compute(&data);
                    hash_updated = true;
                }
                possibly_moved_files.insert(hash.clone(), (path, stage_status));
                continue;
            }
        } else {
            // File is new (does not appear in index)
            // Since it is untracked, we just say it's new and know nothing about it
            Status::New
        };

        // Adding file statuses that are not new, moved or deleted
        file_statuses.push(FileWithStatus {
            path,
            status,
            stage_status,
        })
    }

    let mut new_file_data: Option<(PathBuf, StageStatus)>;

    // File is deleted or moved but still in index so the change it's not staged for commit
    for (path, hashc) in index_data.into_iter() {
        // There is no use on checking deleted files on both maps, by removing every file in the
        // index from the commit data we get the files that only appear in the commit.
        commit_data.remove(&path);

        new_file_data = possibly_moved_files.remove(&hashc.hash);

        file_statuses.push(match new_file_data {
            // New file has the same hash as a deleted file, so the deleted file has just been moved.
            Some((new_path, _)) => FileWithStatus {
                path: new_path,
                status: Status::Moved { previous: path },
                stage_status: StageStatus::NotCommit,
            },
            None => FileWithStatus {
                path,
                status: Status::Deleted,
                stage_status: StageStatus::NotCommit,
            },
        });
    }

    // Deleted or moved file does not appear in index (we can assume the new name of a moved file
    // appears in the index) so the changes are staged for commit.
    for (path, hash) in commit_data.into_iter() {
        new_file_data = possibly_moved_files.remove(&hash);

        file_statuses.push(match new_file_data {
            // New file has the same hash as a deleted file, so the deleted file has just been moved.
            Some((new_path, _)) => FileWithStatus {
                path: new_path,
                status: Status::Moved { previous: path },
                stage_status: StageStatus::Commit,
            },
            None => FileWithStatus {
                path,
                status: Status::Deleted,
                stage_status: StageStatus::Commit,
            },
        });
    }

    // Finally, we add the new files.
    for (path, stage_status) in possibly_moved_files.into_values() {
        file_statuses.push(FileWithStatus {
            path,
            status: Status::New,
            stage_status,
        });
    }

    file_statuses
}

fn read_working_tree_data() -> Result<Vec<FileData>> {
    let all_files = fs::read_not_ignored_paths(&Constants::working_tree_root_path())
        .context("could not get files in working tree")?;

    // Getting data  from working tree
    let working_tree = fs::object::as_blob_objects(all_files)
        .context("could not read files in working tree as objects")?;

    let mut working_tree_data: Vec<FileData> = Vec::new();
    for mut o in working_tree {
        let metadata = std::fs::metadata(&o.path);
        let cache = if let Ok(m) = metadata {
            IndexEntryCache::try_from(m).warn_unwrap()
        } else {
            IndexEntryCache::default()
        };
        working_tree_data.push(FileData {
            path: std::mem::take(&mut o.path),
            data: o.data(),
            cache,
        });
    }

    Ok(working_tree_data)
}

type CommitData = HashMap<PathBuf, Hash>;

fn read_commit_data() -> Result<CommitData> {
    let mut commit_data = HashMap::new();

    // checking if there is a previous commit, otherwise we just leave the data related to the
    // commit
    let previous_commit_hash =
        fs::get_last_commit_hash().context("could not get last commit hash")?;

    if previous_commit_hash.is_none() {
        log::debug!("no previous commit found");
        return Ok(commit_data);
    }

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

    Ok(commit_data)
}

#[derive(Debug, Default, std::hash::Hash, PartialEq, Eq)]
struct HashCachePair {
    hash: Hash,
    cache: IndexEntryCache,
}
type IndexData = HashMap<PathBuf, HashCachePair>;

fn read_index_data() -> Result<IndexData> {
    let mut index_data = IndexData::default();

    let index = fs::index::read_index_file().context("could not read index file")?;
    let mut hash: Hash;
    let mut cache: IndexEntryCache;
    let mut path: PathBuf;
    for mut ie in index.into_entries() {
        hash = ie.object_hash();
        cache = std::mem::take(&mut ie.cache_data);
        path = ie.into_path();

        index_data.insert(path, HashCachePair { hash, cache });
    }

    Ok(index_data)
}

/// Given a list of file statuses, returns a formatted string depicting this status for every file.
fn format_status(status: Vec<FileWithStatus>) -> String {
    let filtered_status: Vec<FileWithStatus> = status
        .into_iter()
        .filter(|fws| fws.status != Status::Unchanged)
        .collect();

    let mut header = format!(
        "On branch {}",
        fs::get_current_branch_name().unwrap_or("!".into())
    );

    if filtered_status.is_empty() {
        return format!("{}\nThere is nothing to commit, all clean!\n", header);
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
            Status::Moved { previous } => format!(
                "\tmoved:\t{} -> {}\n",
                previous.to_string_lossy().to_string(),
                path_str
            ),
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
