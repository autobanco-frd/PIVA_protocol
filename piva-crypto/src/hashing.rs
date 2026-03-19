//! # Hashing Primitives
//! 
//! Provides SHA-3 (Keccak) and BLAKE3 hashing functions optimized
//! for 512 MB RAM environments.

use std::io::Read;

/// Compute SHA-3 256-bit hash using tiny-keccak
pub fn hash_sha3_256(data: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Sha3};
    
    let mut hasher = Sha3::v256();
    hasher.update(data);
    let mut output = [0u8; 32];
    hasher.finalize(&mut output);
    output
}

/// Compute BLAKE3 hash
pub fn hash_blake3(data: &[u8]) -> [u8; 32] {
    use blake3::Hasher;
    
    let mut hasher = Hasher::new();
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

/// Compute BLAKE3 hash for streaming data without loading full content into RAM
pub fn hash_blake3_stream(mut reader: impl Read) -> std::io::Result<[u8; 32]> {
    use blake3::Hasher;
    
    let mut hasher = Hasher::new();
    let mut buffer = [0u8; 8192]; // 8KB buffer to respect 512 MB limit
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(*hasher.finalize().as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sha3_deterministic() {
        let data = b"PIVA Protocol Test Data";
        let hash1 = hash_sha3_256(data);
        let hash2 = hash_sha3_256(data);
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_blake3_deterministic() {
        let data = b"PIVA Protocol Test Data";
        let hash1 = hash_blake3(data);
        let hash2 = hash_blake3(data);
        assert_eq!(hash1, hash2);
    }
    
    #[test]
    fn test_stream_hash() {
        use std::io::Cursor;
        
        let data = b"PIVA Protocol Test Data";
        let cursor = Cursor::new(data);
        let hash_stream = hash_blake3_stream(cursor).unwrap();
        let hash_direct = hash_blake3(data);
        assert_eq!(hash_stream, hash_direct);
    }
    
    #[test]
    fn test_different_hashes() {
        let data = b"PIVA Protocol Test Data";
        let sha3_hash = hash_sha3_256(data);
        let blake3_hash = hash_blake3(data);
        
        // They should be different (different algorithms)
        assert_ne!(sha3_hash, blake3_hash);
    }
}
