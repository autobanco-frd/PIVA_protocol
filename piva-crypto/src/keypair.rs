//! # Keypair Management
//! 
//! Ed25519 keypair wrapper for signing and verification operations.

use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeypairError {
    #[error("Invalid keypair format")]
    InvalidFormat,
    #[error("Signature verification failed")]
    VerificationFailed,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Ed25519 keypair wrapper
#[derive(Debug, Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        Self {
            signing_key,
            verifying_key,
        }
    }
    
    /// Create keypair from existing signing key
    pub fn from_signing_key(signing_key: SigningKey) -> Self {
        let verifying_key = signing_key.verifying_key();
        Self {
            signing_key,
            verifying_key,
        }
    }
    
    /// Sign data and return signature
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }
    
    /// Verify signature against data using public key
    pub fn verify(public_key: &[u8; 32], data: &[u8], signature: &[u8; 64]) -> Result<(), KeypairError> {
        let verifying_key = VerifyingKey::from_bytes(public_key)
            .map_err(|_| KeypairError::InvalidFormat)?;
        
        let signature = Signature::from_bytes(signature);
        
        verifying_key
            .verify(data, &signature)
            .map_err(|_| KeypairError::VerificationFailed)
    }
    
    /// Get the public key bytes
    pub fn public_key(&self) -> [u8; 32] {
        self.verifying_key.to_bytes()
    }
    
    /// Get the secret key bytes
    pub fn secret_key(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }
    
    /// Get the public key as VerifyingKey
    pub fn verifying_key(&self) -> VerifyingKey {
        self.verifying_key
    }
    
    /// Get the signing key as SigningKey
    pub fn signing_key(&self) -> &SigningKey {
        &self.signing_key
    }
    
    /// Serialize keypair to bytes (secret + public)
    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&self.signing_key.to_bytes());
        bytes[32..].copy_from_slice(&self.verifying_key.to_bytes());
        bytes
    }
    
    /// Deserialize keypair from bytes
    pub fn from_bytes(bytes: &[u8; 64]) -> Result<Self, KeypairError> {
        let secret_key_array: [u8; 32] = bytes[..32].try_into()
            .map_err(|_| KeypairError::InvalidFormat)?;
        let public_key_array: [u8; 32] = bytes[32..].try_into()
            .map_err(|_| KeypairError::InvalidFormat)?;
        
        let signing_key = SigningKey::from_bytes(&secret_key_array);
        let verifying_key = VerifyingKey::from_bytes(&public_key_array)
            .map_err(|_| KeypairError::InvalidFormat)?;
        
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }
}

/// Serializable keypair for storage/transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableKeyPair {
    pub secret_key: [u8; 32],
    pub public_key: [u8; 32],
}

impl From<&KeyPair> for SerializableKeyPair {
    fn from(keypair: &KeyPair) -> Self {
        Self {
            secret_key: keypair.secret_key(),
            public_key: keypair.public_key(),
        }
    }
}

impl TryFrom<SerializableKeyPair> for KeyPair {
    type Error = KeypairError;
    
    fn try_from(serializable: SerializableKeyPair) -> Result<Self, Self::Error> {
        let signing_key = SigningKey::from_bytes(&serializable.secret_key);
        let verifying_key = VerifyingKey::from_bytes(&serializable.public_key)
            .map_err(|_| KeypairError::InvalidFormat)?;
        
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let pubkey = keypair.public_key();
        let seckey = keypair.secret_key();
        
        assert_ne!(&pubkey[..], &[0u8; 32]);
        assert_ne!(&seckey[..], &[0u8; 32]);
    }
    
    #[test]
    fn test_sign_verify() {
        let keypair = KeyPair::generate();
        let data = b"PIVA Protocol test message";
        
        let signature = keypair.sign(data);
        let pubkey = keypair.public_key();
        
        // Verify with correct signature
        assert!(KeyPair::verify(&pubkey, data, &signature.to_bytes()).is_ok());
        
        // Verify with wrong data
        assert!(KeyPair::verify(&pubkey, b"wrong data", &signature.to_bytes()).is_err());
        
        // Verify with wrong public key
        let wrong_pubkey = [1u8; 32];
        assert!(KeyPair::verify(&wrong_pubkey, data, &signature.to_bytes()).is_err());
    }
    
    #[test]
    fn test_serialization_roundtrip() {
        let keypair = KeyPair::generate();
        let bytes = keypair.to_bytes();
        let restored = KeyPair::from_bytes(&bytes).unwrap();
        
        assert_eq!(keypair.public_key(), restored.public_key());
        assert_eq!(keypair.secret_key(), restored.secret_key());
    }
    
    #[test]
    fn test_serializable_keypair() {
        let keypair = KeyPair::generate();
        let serializable: SerializableKeyPair = (&keypair).into();
        let restored: KeyPair = serializable.try_into().unwrap();
        
        assert_eq!(keypair.public_key(), restored.public_key());
        
        // Test that signatures match
        let data = b"test message";
        let original_sig = keypair.sign(data);
        let restored_sig = restored.sign(data);
        assert_eq!(original_sig.to_bytes(), restored_sig.to_bytes());
    }
}
