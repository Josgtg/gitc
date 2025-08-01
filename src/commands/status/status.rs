use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use anyhow::{Context, Result};

use super::format::format_status;
use super::read::*;

use crate::byteable::Byteable;
use crate::error::WarnUnwrap;
use crate::hashing::Hash;
use crate::index::IndexEntryCache;
use crate::object::Object;

pub struct FileWithStatus {
    pub path: PathBuf,
    pub status: Status,
    pub stage_status: StageStatus,
}

pub struct FileData {
    pub path: PathBuf,
    pub reader: BufReader<File>,
    pub cache: IndexEntryCache,
}

#[derive(Eq, PartialEq, Debug)]
pub enum Status {
    New,
    Modified,
    Deleted,
    Moved { previous: PathBuf },
    Unchanged,
}

#[derive(Eq, PartialEq, Debug)]
pub enum StageStatus {
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
    let commit_data_opt = read_commit_data().context("could not get commit data")?;
    let no_commits = commit_data_opt.is_none();
    let commit_data = commit_data_opt.unwrap_or_default();

    let index_data = read_index_data().context("could not get index data")?;

    let working_tree_data = read_working_tree_data().context("could not get working tree data")?;

    let file_statuses = determine_statuses(commit_data, index_data, working_tree_data);

    Ok(format_status(file_statuses, no_commits))
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

    let mut file_hash: Hash = Hash::default();
    // Used to avoid hashing twice in the same iteration
    let mut hash_computed: bool;

    // Helper function
    fn hash_if_not_computed(
        file_hash: &mut Hash,
        file_reader: &mut BufReader<File>,
        hash_computed: &mut bool,
    ) -> Result<()> {
        if *hash_computed {
            return Ok(());
        }

        let mut file_data = Vec::new();
        file_reader
            .read_to_end(&mut file_data)
            .context("could not read file contents")?;

        let blob = Object::from_bytes_new_blob(&file_data);
        let blob_bytes = blob.as_bytes().context("could not encode as blob object")?;

        *hash_computed = true;
        *file_hash = Hash::compute(&blob_bytes);

        Ok(())
    }

    let mut status: Status;
    let mut stage_status: StageStatus;
    let mut index_hash: Hash = Hash::default();
    for FileData {
        path: file_path,
        reader: mut file_reader,
        cache: file_cache,
    } in working_tree_data
    {
        hash_computed = false;

        // Notice the use of the `remove` function here instead of the `get` one, this way we can
        // keep track of the files that appear on the index but not in the working tree, since the
        // files left at the end of the loop are only in the index. Same with `commit_data::remove`
        // below.
        stage_status = if let Some((index_hash, index_cache)) = index_data.remove(&file_path) {
            // Path in index
            if index_cache.matches_loose(&file_cache) {
                // And metadata shows it's unchanged; we have the current version of the file
                // in the index so it is staged for commit
                StageStatus::Commit
            } else {
                // If checking with cache is not successful, we hash the file data and run the
                // checks with the hash
                hash_if_not_computed(&mut file_hash, &mut file_reader, &mut hash_computed)
                    .warn_unwrap_or_default();
                if index_hash == file_hash {
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

        status = if let Some(commit_hash) = commit_data.remove(&file_path) {
            if stage_status == StageStatus::Commit && commit_hash == index_hash {
                // We can only check this for files with the Commit stage status since those files
                // are stored in the index in it's current version, otherwise we could be checking
                // for an older version of a file
                Status::Unchanged
            } else {
                // Otherwise, we compare the file in the working tree, with the previous commit
                hash_if_not_computed(&mut file_hash, &mut file_reader, &mut hash_computed)
                    .warn_unwrap_or_default();
                if commit_hash == file_hash {
                    // Same data than previous commit, the file is unchanged
                    Status::Unchanged
                } else {
                    // Different data, the file has been modified
                    Status::Modified
                }
            }
        } else {
            if stage_status == StageStatus::Untracked {
                // It's new but not tracked so we don't care
                Status::New
            } else {
                // File is new, but it is being tracked so we detail it's status
                hash_if_not_computed(&mut file_hash, &mut file_reader, &mut hash_computed)
                    .warn_unwrap_or_default();
                possibly_moved_files.insert(file_hash.clone(), (file_path, stage_status));
                continue;
            }
        };

        // Adding file statuses that are not new, moved or deleted
        file_statuses.push(FileWithStatus {
            path: file_path,
            status,
            stage_status,
        })
    }

    // Processing deleted or moved files

    let mut moved_file_data: Option<(PathBuf, StageStatus)>;
    for (path, hashc) in index_data.into_iter() {
        // There is no use on checking deleted files on both maps, by removing every file in the
        // index from the commit data we get the files that only appear in the commit.
        commit_data.remove(&path);

        index_hash = hashc.0;
        moved_file_data = possibly_moved_files.remove(&index_hash);

        file_statuses.push(match moved_file_data {
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
        moved_file_data = possibly_moved_files.remove(&hash);

        file_statuses.push(match moved_file_data {
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
