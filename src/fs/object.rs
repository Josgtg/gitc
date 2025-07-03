use std::ffi::OsStr;
use std::fs;

use crate::byteable::Byteable;
use crate::error::CustomResult;
use crate::hashing::Hash;
use crate::object::Object;
use crate::utils::zlib;
use crate::{Constants, Result};

/// Writes a serialized and compressed version of the object to the folder in `Constants::object_dir`,
/// returning the hash used to find said object.
///
/// # Errors
///
/// This function can fail if there was not possible to create and write to the file or the object
/// couldn't be compressed.
pub fn write_to_object_dir(object: Object) -> Result<Hash> {
    let data = object
        .as_bytes()
        .map_err_with("could not encode object data when writing to object dir")?;
    let compressed = zlib::compress(data.as_ref())
        .map_err_with("could not compress object when trying to write to object dir")?;
    let hash = Hash::new(data.as_ref());

    let hash_str = hash.to_string();
    let file_dir = &hash_str[0..2];
    let file_name = &hash_str[2..];

    let folder_path = Constants::objects_path().join(OsStr::new(file_dir));
    let file_path = folder_path.join(OsStr::new(file_name));

    fs::create_dir_all(folder_path)?;

    // avoiding writing to an already existing file
    if fs::exists(&file_path)
        .map_err_with("could not check for object file existance when writing to object dir")?
    {
        return Ok(hash);
    }

    fs::write(&file_path, compressed)
        .map_err_with(format!("could not write to object file: {file_path:?}"))?;

    Ok(hash)
}
