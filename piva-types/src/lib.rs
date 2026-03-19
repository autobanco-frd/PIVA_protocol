//! # Shared Types for PIVA Project
//! 
//! Common types shared across multiple crates to avoid circular dependencies.

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssetError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid ID")]
    InvalidId,
}

/// Network modes for PIVA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    Devnet,
    Testnet,
    Mainnet,
}

impl NetworkMode {
    pub fn port(&self) -> u16 {
        match self {
            NetworkMode::Devnet => 7800,
            NetworkMode::Testnet => 7801,
            NetworkMode::Mainnet => 7802,
        }
    }
    
    pub fn magic_byte(&self) -> u8 {
        match self {
            NetworkMode::Devnet => 0x01,
            NetworkMode::Testnet => 0x02,
            NetworkMode::Mainnet => 0x03,
        }
    }
    
    pub fn max_connections(&self) -> usize {
        match self {
            NetworkMode::Devnet => 5,
            NetworkMode::Testnet => 25,
            NetworkMode::Mainnet => 50,
        }
    }
    
    pub fn buffer_size(&self) -> usize {
        match self {
            NetworkMode::Devnet => 4096,   // 4 KB
            NetworkMode::Testnet => 8192,  // 8 KB
            NetworkMode::Mainnet => 8192, // 8 KB
        }
    }
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::Devnet => write!(f, "Devnet"),
            NetworkMode::Testnet => write!(f, "Testnet"),
            NetworkMode::Mainnet => write!(f, "Mainnet"),
        }
    }
}

impl std::str::FromStr for NetworkMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "devnet" => Ok(NetworkMode::Devnet),
            "testnet" => Ok(NetworkMode::Testnet),
            "mainnet" => Ok(NetworkMode::Mainnet),
            _ => Err(format!("Invalid network mode: {}", s)),
        }
    }
}

/// Asset types supported by PIVA
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    PropertyTitle,
    Diploma,
    LegalDocument,
    CommercialOffer,
    AudioMusic,
}

impl AssetType {
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

impl std::str::FromStr for AssetType {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetMetadata {
    pub asset_type: AssetType,
    pub issuer_pubkey: [u8; 32],
    pub created_at: u64,
    pub description: String,
    pub custom_fields: BTreeMap<String, String>,
}

/// Asset ID structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetId {
    pub hash: [u8; 32],
}

impl AssetId {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        if bytes.len() != 32 {
            return Err("Invalid hash length".into());
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(bytes);
        Ok(Self { hash })
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.hash
    }
    
    pub fn to_string(&self) -> String {
        hex::encode(self.hash)
    }
}

/// Asset entry structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: AssetId,
    pub metadata: AssetMetadata,
    pub content_hash: [u8; 32],
    pub content_size: u64,
    #[serde(with = "BigArray")]
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
        signing_keypair: &piva_crypto::KeyPair,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // Create ID from metadata
        let id_hash = piva_crypto::hash_sha3_256(&serde_json::to_vec(&metadata)?);
        let id = AssetId { hash: id_hash };
        
        // Sign the asset ID
        let signature = signing_keypair.sign(&id_hash);
        
        Ok(Self {
            id,
            metadata,
            content_hash,
            content_size,
            signature: signature.to_bytes(),
            network,
        })
    }
    
    /// Convert to bytes for storage
    pub fn to_bytes(&self) -> Result<Vec<u8>, AssetError> {
        bincode::serialize(self).map_err(|e| AssetError::Serialization(e.to_string()))
    }
    
    /// Convert from bytes from storage
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, AssetError> {
        bincode::deserialize(bytes).map_err(|e| AssetError::Serialization(e.to_string()))
    }
    
    /// Verify the integrity of the asset entry
    pub fn verify_integrity(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Verify the asset ID matches the metadata
        let expected_id_hash = piva_crypto::hash_sha3_256(&serde_json::to_vec(&self.metadata)?);
        if self.id.hash != expected_id_hash {
            return Err("Asset ID mismatch".into());
        }
        
        // Verify signature using the static method
        piva_crypto::KeyPair::verify(
            &self.metadata.issuer_pubkey, 
            &self.id.hash, 
            &self.signature
        ).map_err(|e| format!("Firma inválida: {}", e))?;
        
        Ok(())
    }
}
