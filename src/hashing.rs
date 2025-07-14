use sha1::{Digest, Sha1};
use std::{fmt::Display, rc::Rc, str::FromStr};
use anyhow::Result;

#[derive(Debug, PartialEq, Eq, std::hash::Hash, Clone)]
pub struct Hash(Rc<[u8; 20]>);

impl Hash {
    /// Returns the SHA1 hash for the data passed
    pub fn new(value: &[u8]) -> Self {
        let mut hasher = Sha1::new();
        hasher.update(value);
        Hash(Rc::new(hasher.finalize().into()))
    }
}

impl From<[u8; 20]> for Hash {
    fn from(value: [u8; 20]) -> Self {
        Self(Rc::new(value))
    }
}

impl From<&Rc<[u8; 20]>> for Hash {
    fn from(value: &Rc<[u8; 20]>) -> Self {
        Self(Rc::clone(value))
    }
}

impl From<Hash> for [u8; 20] {
    fn from(value: Hash) -> Self {
        *value.0
    }
}

impl FromStr for Hash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let hash = hex::decode(s)?;
        Ok(Hash::new(&hash))
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoded = hex::encode(self.as_ref());
        f.write_str(encoded.as_str())
    }
}

// Tests

#[cfg(test)]
mod tests {
    use crate::hashing::Hash;

    #[test]
    pub fn test_hashing() {
        let data = b"this is binary data";
        let data_hash = Hash::new(data);
        let data2 = b"this is binary data";
        let data2_hash = Hash::new(data2);
        assert_eq!(data_hash, data2_hash);
        let data3 = b"This is binary data";
        let data3_hash = Hash::new(data3);
        assert_ne!(data_hash, data3_hash);
    }

    #[test]
    pub fn test_no_change() {
        let hash_bytes: [u8; 20] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let hash = Hash::from(hash_bytes);
        let hash_bytes_changed: [u8; 20] = hash.into();
        assert_eq!(hash_bytes, hash_bytes_changed);
    }
}
