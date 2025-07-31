use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::byteable::Byteable;
use crate::hashing::Hash;
use crate::object::Object;
use crate::{Constants, utils};

/// Given an object, gets it's serialized representation and hash, and writes it to the object
/// directory.
///
/// If the encoded object and their hash has already been computed, it's better to use the
/// `write_to_object_dir` function since it does not calculate them from scratch.
///
/// # Errors
///
/// This function will fail if the object could not be encoded or the data could not be written.
pub fn write_object(object: &Object) -> Result<Hash> {
    let bytes = object.as_bytes().context("could not encode object")?;
    let hash = Hash::compute(&bytes);

    write_to_object_dir(&bytes, &hash).context("could not write to object directory")?;

    Ok(hash)
}

/// Returns the directory in the first position and the filename in the second one.
pub fn get_object_hash_and_filename(hash_str: &str) -> (&str, &str) {
    let file_dir = &hash_str[0..2];
    let file_name = &hash_str[2..];
    (file_dir, file_name)
}

/// Writes the given bytes as an object in the objects
/// directory, compressing them using zlib.
///
/// Use the `write_object` function if you have not already computed the bytes or the hash of an
/// object.
///
/// # Errors
///
/// This function can fail if there was not possible to create and write to the file.
pub fn write_to_object_dir(bytes: &[u8], hash: &Hash) -> Result<()> {
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

    let compressed = utils::zlib::compress(bytes).context("could not compress object data")?;

    let mut file = std::fs::File::create(&file_path).context("could not create object file")?;
    let mut permissions = file
        .metadata()
        .context("could not get file metadata")?
        .permissions();
    permissions.set_readonly(true);
    file.set_permissions(permissions)
        .context("could not set file permissions")?;

    file.write_all(&compressed)
        .context(format!("could not write to object file: {:?}", file_path))?;

    Ok(())
}

/// Looks for the file inside the objects directory that has the given hash and converts it to an
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
    let decompressed = utils::zlib::decompress(&bytes).context("could not decompress bytes")?;

    Object::from_bytes(&decompressed).context("could not create object from file bytes")
}

/// Reads all the given paths, reading the file and converting it to a `BufBlob` object, which
/// stores an object's path.
///
/// This function will call itself recursively if a path is from a directory.
///
/// # Errors
///
/// This function can fail if there was an error while reading a file or when creating the object.
#[allow(unused)]
pub fn as_objects(paths: Vec<PathBuf>) -> Result<Vec<Object>> {
    let mut objects = Vec::with_capacity(paths.len());
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut bytes: Vec<u8>;
    for path in paths {
        if path.is_dir() {
            dirs.push(path);
            continue;
        }
        bytes = std::fs::read(&path).context(format!("could not read file {:?}", path))?;
        objects.push(Object::from_bytes_new_blob(&bytes));
    }

    // Calling recursively for every directory
    let mut subdirs: Vec<PathBuf>;
    for d in dirs {
        subdirs = super::path::read_all_dir_paths(&d).context(format!(
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
