//! # Asset Data Structures
//! 
//! Core data structures for representing RWA (Real World Assets) in the PIVA protocol.

use crate::network::NetworkMode;
use piva_crypto::{hash_sha3_256, KeyPair};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetError {
    #[error("Invalid asset ID format")]
    InvalidId,
    #[error("Asset type mismatch")]
    TypeMismatch,
    #[error("Signature verification failed")]
    InvalidSignature,
    #[error("Content hash mismatch")]
    ContentHashMismatch,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Asset type enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetType {
    PropertyTitle,
    Diploma,
    LegalDocument,
    CommercialOffer,
    AudioMusic,
}

impl AssetType {
    /// Get the string representation for display
    pub fn as_str(&self) -> &'static str {
        match self {
            AssetType::PropertyTitle => "property_title",
            AssetType::Diploma => "diploma",
            AssetType::LegalDocument => "legal_document",
            AssetType::CommercialOffer => "commercial_offer",
            AssetType::AudioMusic => "audio_music",
        }
    }
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AssetType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "property_title" => Ok(AssetType::PropertyTitle),
            "diploma" => Ok(AssetType::Diploma),
            "legal_document" => Ok(AssetType::LegalDocument),
            "commercial_offer" => Ok(AssetType::CommercialOffer),
            "audio_music" => Ok(AssetType::AudioMusic),
            _ => Err(format!("Invalid asset type: {}", s)),
        }
    }
}

/// Asset metadata structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub asset_type: AssetType,
    pub issuer_pubkey: [u8; 32],
    pub created_at: u64,
    pub description: String,
    pub custom_fields: BTreeMap<String, String>,
}

impl AssetMetadata {
    /// Create new asset metadata
    pub fn new(
        asset_type: AssetType,
        issuer_pubkey: [u8; 32],
        description: String,
    ) -> Self {
        Self {
            asset_type,
            issuer_pubkey,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            description,
            custom_fields: BTreeMap::new(),
        }
    }
    
    /// Add a custom field
    pub fn with_custom_field(mut self, key: String, value: String) -> Self {
        self.custom_fields.insert(key, value);
        self
    }
    
    /// Get serialized bytes for hashing
    pub fn to_bytes(&self) -> Result<Vec<u8>, AssetError> {
        bincode::serialize(self)
            .map_err(|e| AssetError::SerializationError(e.to_string()))
    }
}

/// Asset ID - SHA-3 hash of metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetId(String);

impl AssetId {
    /// Create new asset ID from metadata
    pub fn from_metadata(metadata: &AssetMetadata, network: NetworkMode) -> Result<Self, AssetError> {
        let metadata_bytes = metadata.to_bytes()?;
        let hash = hash_sha3_256(&metadata_bytes);
        let hash_hex = hex::encode(hash);
        Ok(Self(format!("{}{}", network.prefix(), hash_hex)))
    }
    
    /// Get the raw hash part (without prefix)
    pub fn hash(&self) -> Result<[u8; 32], AssetError> {
        let hash_part = self.0
            .strip_prefix("piva_dev_")
            .or_else(|| self.0.strip_prefix("piva_test_"))
            .or_else(|| self.0.strip_prefix("piva_live_"))
            .ok_or(AssetError::InvalidId)?;
        
        hex::decode(hash_part)
            .map_err(|_| AssetError::InvalidId)?
            .try_into()
            .map_err(|_| AssetError::InvalidId)
    }
    
    /// Get the network mode from the ID
    pub fn network(&self) -> Result<NetworkMode, AssetError> {
        if self.0.starts_with("piva_dev_") {
            Ok(NetworkMode::Devnet)
        } else if self.0.starts_with("piva_test_") {
            Ok(NetworkMode::Testnet)
        } else if self.0.starts_with("piva_live_") {
            Ok(NetworkMode::Mainnet)
        } else {
            Err(AssetError::InvalidId)
        }
    }
}

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for AssetId {
    type Err = AssetError;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = Self(s.to_string());
        // Validate by trying to extract network
        id.network()?;
        Ok(id)
    }
}

/// Complete asset entry with all necessary data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: AssetId,
    pub metadata: AssetMetadata,
    pub content_hash: [u8; 32],
    pub content_size: u64,
    #[serde(with = "serde_arrays")]
    pub signature: [u8; 64],
    pub network: NetworkMode,
}

impl AssetEntry {
    /// Create a new asset entry
    pub fn new(
        metadata: AssetMetadata,
        content_hash: [u8; 32],
        content_size: u64,
        network: NetworkMode,
        signing_keypair: &KeyPair,
    ) -> Result<Self, AssetError> {
        let id = AssetId::from_metadata(&metadata, network)?;
        
        // Sign the asset ID
        let id_bytes = id.hash()?;
        let signature = signing_keypair.sign(&id_bytes);
        
        Ok(Self {
            id,
            metadata,
            content_hash,
            content_size,
            signature: signature.to_bytes(),
            network,
        })
    }
    
    /// Verify the integrity of the asset entry
    pub fn verify_integrity(&self) -> Result<(), AssetError> {
        // Verify the asset ID matches the metadata
        let expected_id = AssetId::from_metadata(&self.metadata, self.network)?;
        if self.id != expected_id {
            return Err(AssetError::InvalidId);
        }
        
        // Verify the signature
        let id_hash = self.id.hash()?;
        KeyPair::verify(
            &self.metadata.issuer_pubkey,
            &id_hash,
            &self.signature,
        ).map_err(|_| AssetError::InvalidSignature)?;
        
        Ok(())
    }
    
    /// Get serialized bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, AssetError> {
        bincode::serialize(self)
            .map_err(|e| AssetError::SerializationError(e.to_string()))
    }
    
    /// Create from serialized bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError> {
        bincode::deserialize(bytes)
            .map_err(|e| AssetError::SerializationError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piva_crypto::KeyPair;
    
    #[test]
    fn test_asset_type_from_str() {
        assert_eq!("diploma".parse::<AssetType>().unwrap(), AssetType::Diploma);
        assert_eq!("property_title".parse::<AssetType>().unwrap(), AssetType::PropertyTitle);
        assert!("invalid".parse::<AssetType>().is_err());
    }
    
    #[test]
    fn test_metadata_creation() {
        let pubkey = [42u8; 32];
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            pubkey,
            "Test Diploma".to_string(),
        );
        
        assert_eq!(metadata.asset_type, AssetType::Diploma);
        assert_eq!(metadata.issuer_pubkey, pubkey);
        assert_eq!(metadata.description, "Test Diploma");
        assert!(metadata.created_at > 0);
    }
    
    #[test]
    fn test_asset_id_creation() {
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair.public_key(),
            "Test Diploma".to_string(),
        );
        
        let id = AssetId::from_metadata(&metadata, NetworkMode::Devnet).unwrap();
        assert!(id.0.starts_with("piva_dev_"));
        assert_eq!(id.network().unwrap(), NetworkMode::Devnet);
    }
    
    #[test]
    fn test_asset_entry_creation() {
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair.public_key(),
            "Test Diploma".to_string(),
        );
        let content_hash = [123u8; 32];
        let content_size = 1024;
        
        let entry = AssetEntry::new(
            metadata,
            content_hash,
            content_size,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        assert!(entry.verify_integrity().is_ok());
        assert_eq!(entry.content_hash, content_hash);
        assert_eq!(entry.content_size, content_size);
        assert_eq!(entry.network, NetworkMode::Devnet);
    }
    
    #[test]
    fn test_asset_entry_serialization() {
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair.public_key(),
            "Test Diploma".to_string(),
        );
        let content_hash = [123u8; 32];
        let content_size = 1024;
        
        let entry = AssetEntry::new(
            metadata,
            content_hash,
            content_size,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        let bytes = entry.to_bytes().unwrap();
        let restored = AssetEntry::from_bytes(&bytes).unwrap();
        
        assert_eq!(entry.id, restored.id);
        assert_eq!(entry.content_hash, restored.content_hash);
        assert!(restored.verify_integrity().is_ok());
    }
    
    #[test]
    fn test_invalid_signature() {
        let keypair1 = KeyPair::generate();
        let keypair2 = KeyPair::generate();
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair1.public_key(),
            "Test Diploma".to_string(),
        );
        let content_hash = [123u8; 32];
        let content_size = 1024;
        
        let mut entry = AssetEntry::new(
            metadata,
            content_hash,
            content_size,
            NetworkMode::Devnet,
            &keypair1,
        ).unwrap();
        
        // Corrupt the signature
        entry.signature = keypair2.sign(&entry.id.hash().unwrap()).to_bytes();
        
        assert!(entry.verify_integrity().is_err());
    }
}
