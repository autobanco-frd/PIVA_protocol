//! # PIVA Cryptographic Primitives
//! 
//! This module provides the core cryptographic operations used throughout
//! the PIVA protocol, optimized for 512 MB RAM environments.

pub use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Signature};

pub mod hashing;
pub mod keypair;

pub use hashing::{hash_sha3_256, hash_blake3, hash_blake3_stream};
pub use keypair::KeyPair;

/// Re-export commonly used types
pub type PublicKey = ed25519_dalek::VerifyingKey;
pub type SecretKey = ed25519_dalek::SigningKey;

/// Convenience wrapper for hash function
pub fn hash(data: &[u8]) -> [u8; 32] {
    hash_blake3(data)
}

/// Convenience wrapper for signature creation
pub fn create_signature(data: &[u8], private_key: &[u8; 32]) -> Result<[u8; 64], Box<dyn std::error::Error>> {
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(data);
    Ok(signature.to_bytes())
}

/// Convenience wrapper for signature verification
pub fn verify_signature(data: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> Result<(), Box<dyn std::error::Error>> {
    KeyPair::verify(public_key, data, signature)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
}
