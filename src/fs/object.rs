use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::Constants;
use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::object::{ExtendedBlob, Object};
use crate::utils::zlib;

/// Given an object, gets it's compressed representation and hash, and writes it to the object
/// directory.
///
/// If the encoded object and their hash has already been processed, it's better to use the
/// `write_to_object_dir` function since it does not calculate them from scratch.
///
/// # Errors
///
/// This function will fail if the object could not be encoded or the data could not be written.
fn write_object(object: &Object) -> Result<Hash> {
    let bytes = object.as_bytes().context("could not encode object")?;
    let hash = Hash::new(bytes.as_ref());

    write_to_object_dir(&bytes, &hash).context("could not write to object directory")?;

    Ok(hash)
}

/// Writes a serialized and compressed version of the object to the folder in `Constants::object_dir`,
/// returning the hash used to find said object.
///
/// Unlike `write_object`, this function does not realize any extra operations on an object such as getting the
/// hash of the object. It just compresses the bytes given.
///
/// # Errors
///
/// This function can fail if there was not possible to create and write to the file or the object
/// data couldn't be compressed.
pub fn write_to_object_dir(bytes: &[u8], hash: &Hash) -> Result<()> {
    let compressed = zlib::compress(bytes)
        .context("could not compress object when trying to write to object dir")?;

    let hash_str = hash.to_string();
    let file_dir = &hash_str[0..2];
    let file_name = &hash_str[2..];

    let folder_path = Constants::objects_path().join(OsStr::new(file_dir));
    let file_path = folder_path.join(OsStr::new(file_name));

    // avoiding writing to an already existing file
    if fs::exists(&file_path)
        .context("could not check for object file existance when writing to object dir")?
    {
        return Ok(());
    }

    fs::create_dir_all(folder_path)?;

    fs::write(&file_path, compressed)
        .context(format!("could not write to object file: {file_path:?}"))?;

    Ok(())
}

/// Converts the files in the given paths to their `ExtendedObject` representation.
///
/// This function will call itself recursively if a path is from a directory.
///
/// # Errors
///
/// This function can fail if there was an error while reading a file or when creating the object.
pub fn as_objects(paths: Vec<PathBuf>) -> Result<Vec<ExtendedBlob>> {
    let mut objects = Vec::with_capacity(paths.len());
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut bytes: Vec<u8>;
    for p in paths {
        if p.is_dir() {
            dirs.push(p);
            continue;
        }

        // Adding a file
        bytes = fs::read(&p).context(format!("could not read file: {:?}", p))?;
        objects.push(ExtendedBlob {
            object: Object::Blob { data: bytes.into() },
            path: p,
        })
    }

    // Calling recursively for every directory
    let mut subdirs: Vec<PathBuf>;
    for d in dirs {
        subdirs = super::path::read_dir_paths(&d).context(format!(
            "could not read subdirecories for directory: {:?}",
            d
        ))?;
        objects.extend(
            as_objects(subdirs)
                .context(format!("could not get objects from directory: {:?}", d))?,
        );
    }

    Ok(objects)
}
