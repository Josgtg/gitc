use sha1::{Digest, Sha1};

/// Returns the SHA1 hash for the data passed
pub fn hash(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha1::new();
    hasher.update(data);
    hasher.finalize().to_vec()
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
