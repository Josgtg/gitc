use sha1::{Digest, Sha1};
use std::fmt::Display;

#[derive(Debug, PartialEq)]
pub struct Hash([u8; 20]);

impl Hash {
    /// Returns the SHA1 hash for the data passed
    pub fn from(value: &[u8]) -> Self { 
        let mut hasher = Sha1::new();
        hasher.update(value);
        Hash(hasher.finalize().into())
    }
}

impl Into<[u8; 20]> for Hash {
    fn into(self) -> [u8; 20] {
        self.0
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoded = hex::encode(self.as_ref());
        f.write_str(encoded.as_str())
    }
}

impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}


// Tests

#[cfg(test)]
mod tests {
    use crate::hashing::Hash;

    #[test]
    pub fn test_hashing() {
        let data = b"this is binary data";
        let data_hash = Hash::from(data);
        let data2 = b"this is binary data";
        let data2_hash = Hash::from(data2);
        assert_eq!(data_hash, data2_hash);
        let data3 = b"This is binary data";
        let data3_hash = Hash::from(data3);
        assert_ne!(data_hash, data3_hash);
    }
}
