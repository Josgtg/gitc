use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use colored::Colorize;

use crate::Constants;
use crate::byteable::Byteable;
use crate::error::WarnUnwrap;
use crate::fs;
use crate::hashing::Hash;
use crate::index::IndexEntryCache;
use crate::object;
use crate::object::Object;
use crate::object::tree::TreeEntry;

struct FileWithStatus {
    path: PathBuf,
    status: Status,
    stage_status: StageStatus,
}

struct FileData {
    path: PathBuf,
    reader: BufReader<File>,
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
                    .warn_unwrap();
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
                    .warn_unwrap();
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
                    .warn_unwrap();
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

fn read_working_tree_data() -> Result<Vec<FileData>> {
    let filtered_paths = fs::read_not_ignored_paths(&Constants::working_tree_root_path())
        .context("could not filter ignored paths in working tree")?;

    let all_files = fs::path::expand_dirs_from_list(filtered_paths).context("could not get files in working tree")?;

    // Getting data  from working tree
    let working_tree = fs::path::read_bufered(all_files)
        .context("could not read files in working tree as objects")?;

    let working_tree_data = working_tree
        .into_iter()
        .map(|file| FileData {
            path: file.path,
            reader: file.reader,
            cache: file.cache,
        })
        .collect();

    Ok(working_tree_data)
}

type CommitData = HashMap<PathBuf, Hash>;

fn read_commit_data() -> Result<Option<CommitData>> {
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

type IndexData = HashMap<PathBuf, (Hash, IndexEntryCache)>;

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

        index_data.insert(path, (hash, cache));
    }

    Ok(index_data)
}

/// Given a list of file statuses, returns a formatted string depicting this status for every file.
fn format_status(status: Vec<FileWithStatus>, no_commits: bool) -> String {
    let filtered_status: Vec<FileWithStatus> = status
        .into_iter()
        .filter(|fws| fws.status != Status::Unchanged)
        .collect();

    let mut header = format!(
        "On branch {}\n",
        fs::get_current_branch_name().unwrap_or("!".into())
    );

    if no_commits {
        header.push_str("\nNo commits yet\n");
    }

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

        if s.stage_status == StageStatus::Untracked {
            untracked.push_str(&format!("\t{}\n", path_str));
            continue;
        }

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
