use sha1::{Digest, Sha1};

use crate::byteable::Byteable;
use crate::Result;

#[derive(Debug, PartialEq)]
pub struct Hash([u8; 20]);

impl Hash {
    pub fn from_byteable(value: impl Byteable) -> Result<Self> {
        Ok(hash(value.as_bytes()?.as_ref()))
    }
}

impl Into<[u8; 20]> for Hash {
    fn into(self) -> [u8; 20] {
        self.0
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Returns the SHA1 hash for the data passed
pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Sha1::new();
    hasher.update(data);
    Hash(hasher.finalize().into())
}



// Tests

#[cfg(test)]
mod tests {
    use crate::hashing::hash;

    #[test]
    pub fn test_hashing() {
        let data = b"this is binary data";
        let data_hash = hash(data);
        let data2 = b"this is binary data";
        let data2_hash = hash(data2);
        assert_eq!(data_hash, data2_hash);
        let data3 = b"This is binary data";
        let data3_hash = hash(data3);
        assert_ne!(data_hash, data3_hash);
    }
}
