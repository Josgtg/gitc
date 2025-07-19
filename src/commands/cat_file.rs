use std::str::FromStr;

use crate::byteable::Byteable;
use crate::object::Object;
use crate::{hashing::Hash, Constants};
use crate::{fs, utils};

use anyhow::{bail, Context, Result};

pub fn cat_file(hash: &str) -> Result<String> {
    // just checking the string is valid
    let _ = Hash::from_str(hash).context("object hash was invalid")?;

    let (dir, filename) = fs::object::get_object_hash_and_filename(hash);

    let object_path = Constants::objects_path().join(dir).join(filename);

    if !object_path.exists() {
        bail!("hash did not refer to any object")
    }

    let data = std::fs::read(object_path).context("could not read from object file")?;
    let bytes = utils::zlib::decompress(&data).context("could not decompress data")?;

    let object = Object::from_bytes(&bytes).context("could not convert bytes into an object file...")?;

    Ok(format!("{}", object))
}
