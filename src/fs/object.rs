use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::Constants;
use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::object::{BlobExt, Object};
use crate::utils::zlib;

/// Given an object, gets it's serialized representation and hash, and writes it to the object
/// directory.
///
/// If the encoded object and their hash has already been processed, it's better to use the
/// `write_to_object_dir` function since it does not calculate them from scratch.
///
/// # Errors
///
/// This function will fail if the object could not be encoded or the data could not be written.
pub fn write_object(object: &Object) -> Result<Hash> {
    let bytes = object.as_bytes().context("could not encode object")?;
    let hash = Hash::new(bytes.as_ref());

    write_to_object_dir(&bytes, &hash).context("could not write to object directory")?;

    Ok(hash)
}

/// Same as `write_to_object_dir` but this function compresses the bytes before writing them.
pub fn write_blob_to_object_dir(bytes: &[u8], hash: &Hash) -> Result<()> {
    let compressed = zlib::compress(bytes)
        .context("could not compress object when trying to write to object dir")?;

    write_to_object_dir(compressed.as_ref(), hash)
}

/// Returns the directory in the first position and the filename in the second one.
fn get_object_hash_and_filename(hash_str: &str) -> (&str, &str) {
    let file_dir = &hash_str[0..2];
    let file_name = &hash_str[2..];
    (file_dir, file_name)
}

/// Writes the given bytes as an object in the objects
/// directory.
///
/// Use the `write_object` function if you have not already computed the bytes or the hash of an
/// object.
///
/// # Errors
///
/// This function can fail if there was not possible to create and write to the file.
pub fn write_to_object_dir(object_bytes: &[u8], hash: &Hash) -> Result<()> {
    let hash_str = hash.to_string();
    let (file_dir, file_name) = get_object_hash_and_filename(&hash_str);

    let folder_path = Constants::objects_path().join(OsStr::new(file_dir));
    let file_path = folder_path.join(OsStr::new(file_name));

    // avoiding writing to an already existing file
    if fs::exists(&file_path)
        .context("could not check for object file existance when writing to object dir")?
    {
        return Ok(());
    }

    fs::create_dir_all(folder_path)?;

    fs::write(&file_path, object_bytes)
        .context(format!("could not write to object file: {file_path:?}"))?;

    Ok(())
}

/// Looks for the file inside the objects director that hash the given hash and converts it to an
/// object.
///
/// # Errors
///
/// This function can fail if:
/// - The hash did not correspond to any object file.
/// - The file could not be read.
/// - The file data could not be parsed as an object.
pub fn read_object(hash: Hash) -> Result<Object> {
    let hash_str = hash.to_string();
    let (file_dir, file_name) = get_object_hash_and_filename(&hash_str);

    let path = Constants::objects_path().join(file_dir).join(file_name);

    let bytes = fs::read(path).context("could not read file")?;

    Object::from_bytes(&bytes).context("could not create object from file bytes")
}

/// Converts the files in the given paths to their `ExtendedObject` representation.
///
/// This function will call itself recursively if a path is from a directory.
///
/// # Errors
///
/// This function can fail if there was an error while reading a file or when creating the object.
pub fn as_blob_objects(paths: Vec<PathBuf>) -> Result<Vec<BlobExt>> {
    let mut objects = Vec::with_capacity(paths.len());
    let mut dirs: Vec<PathBuf> = Vec::new();
    for p in paths {
        if p.is_dir() {
            dirs.push(p);
            continue;
        }
        objects.push(as_blob_object(p).context("could not create object")?);
    }

    // Calling recursively for every directory
    let mut subdirs: Vec<PathBuf>;
    for d in dirs {
        subdirs = super::path::read_dir_paths(&d).context(format!(
            "could not read subdirecories for directory: {:?}",
            d
        ))?;
        objects.extend(
            as_blob_objects(subdirs)
                .context(format!("could not get objects from directory: {:?}", d))?,
        );
    }

    Ok(objects)
}

/// Converts a file in a path to its blob object representation.
///
/// # Errors
///
/// This function will fail if the file could not be read.
pub fn as_blob_object(path: PathBuf) -> Result<BlobExt> { 
    let bytes = fs::read(&path).context(format!("could not read file: {:?}", &path))?;
    Ok(BlobExt {
        blob: Object::Blob { data: bytes.into() },
        path,
    })
}
