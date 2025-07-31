use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Context, Result};

use crate::byteable::Byteable;
use crate::index::IndexEntryCache;
use crate::object::Object;
use crate::{fs, utils};
use crate::hashing::Hash;
use crate::index::{builder::IndexBuilder, IndexEntry};
use crate::Constants;

const PATTERN_EVERY_FILE: &str = ".";

/// Fetches all files from the worktree (not in .gitignore unless explicitly added),
/// creates blob objects for all of them, creates index entries from those objects
/// and adds them to the index file.
pub fn add(files: &[OsString]) -> Result<String> {
    let root_path = Constants::working_tree_root_path();
    let mut delete_files = false;

    let filtered_paths: Vec<PathBuf> = if files[0] == PATTERN_EVERY_FILE {
        // We only delete files if we are checking every file in the working tree, that way we know
        // if any files are missing (deleted)
        delete_files = true;
        fs::read_not_ignored_paths(&root_path).context("could not filter ignored files")?
    } else {
        // We do not check if a file is in .gitignore if it's deliberately added
        let mut filtered_paths = Vec::new();

        // "normalizing" every path
        let mut canonical: PathBuf;
        let mut relative: PathBuf;
        for f in files {
            // normalizing all paths
            canonical = PathBuf::from(f).canonicalize().context("could not canonicalize path")?;
            relative = utils::path::relative_path(&canonical, &root_path).unwrap_or(canonical);
            filtered_paths.push(relative);
        }

        filtered_paths
    };

    if filtered_paths.is_empty() {
        return Ok("There were no files to add\n".into())
    }

    // reading all files as blob objects
    let objects = fs::path::read_bufered(filtered_paths).context("could not get bufered files")?;

    // getting previous index to update it
    let previous_index = fs::index::read_index_file().context("could not read index file")?;

    // building a set containing hashes already in index to avoid adding a file twice
    let mut index_data: HashMap<PathBuf, (Hash, IndexEntryCache)> = HashMap::new();
    for ie in previous_index.entries() {
        index_data.insert(
            ie.path().to_owned(),
            (ie.object_hash(), ie.cache_data.clone())
        );
    }

    fn hash_from_reader(reader: &mut BufReader<File>) -> Result<(Rc<[u8]>, Hash)> {
        // reading bytes from the file one at a time
        let mut file_bytes = Vec::new();
        reader.read_to_end(&mut file_bytes).context("could not read file contents")?;

        let blob = Object::from_bytes_new_blob(&file_bytes);

        let bytes = blob
            .as_bytes()
            .context("could not encode object for file")?;

        let hash = Hash::compute(bytes.as_ref());

        Ok((bytes, hash))
    }

    // adding index entries
    let mut index_builder = IndexBuilder::from(previous_index);

    let mut index_entry: IndexEntry;
    let mut bytes: Rc<[u8]> = Rc::default();
    let mut hash: Hash = Hash::default();
    let mut hash_computed: bool;
    for mut o in objects {
        hash_computed = false;

        if let Some((index_hash, index_cache)) = index_data.remove(&o.path) {
            // file already in index, we delete it since we won't be needing it and it would be
            // useful later (paths left at the end are deleted files)
            if index_cache.matches_loose(&o.cache) {
                // we can assume the file is unchanged
                continue;
            } else {
                (bytes, hash) = hash_from_reader(&mut o.reader).context(format!("could not hash file: {:?}", o.path))?;
                hash_computed = true;

                if index_hash == hash {
                    // ...and has been modified. We remove it and add it as if it was a new file
                    index_builder.remove_index_entry_by_path(&o.path);
                } else { 
                    // ...and is unchanged, we just ignore it
                    continue;
                }
            }
        }

        // if the `if` above is not triggered, this is a new file (or modified, but we handle it as new)
        
        if !hash_computed {
            (bytes, hash) = hash_from_reader(&mut o.reader).context(format!("could not hash file: {:?}", o.path))?;
        }

        index_entry = IndexEntry::try_from_file(&o.path, hash.clone()).context(format!(
            "could not create index entry from file: {:?}",
            o.path
        ))?;

        fs::object::write_to_object_dir(&bytes, &hash).context("could not write to object dir")?;

        index_builder.add_index_entry(index_entry);
    }

    if delete_files {
        for p in index_data.into_keys() {
            index_builder.remove_index_entry_by_path(&p);
        }
    }

    let index = index_builder.build();

    fs::index::write_index_file(index).context("could not write to index file")?;

    Ok("Added files successfully\n".into())
}
