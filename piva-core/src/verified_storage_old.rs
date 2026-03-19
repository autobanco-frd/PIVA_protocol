//! # Verified Asset Retrieval
//! 
//! Extension trait for verified asset retrieval with signature validation.

use crate::{asset::AssetEntry, network::NetworkMode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerificationError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Asset not found")]
    NotFound,
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Network mode mismatch")]
    NetworkMismatch,
    #[error("Asset corrupted: {0}")]
    Corruption(String),
}

/// Trait for storage verification operations
pub trait VerifiedStorage {
    /// Get asset and verify its signature integrity
    fn get_asset_verified(&self, asset_id: &str) -> Result<AssetEntry, VerificationError>;
    
    /// Store asset with signature pre-validation
    fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError>;
    
    /// Get network mode
    fn network_mode(&self) -> NetworkMode;
}

/// Simple verification implementation for testing
pub struct MockVerifiedStorage {
    network_mode: NetworkMode,
}

impl MockVerifiedStorage {
    pub fn new(network_mode: NetworkMode) -> Self {
        Self { network_mode }
    }
}

impl VerifiedStorage for MockVerifiedStorage {
    fn get_asset_verified(&self, _asset_id: &str) -> Result<AssetEntry, VerificationError> {
        // Mock implementation - in real usage this would interface with Storage
        Err(VerificationError::NotFound)
    }
    
    fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError> {
        // 1. Pre-validate signature before storing
        asset.verify_integrity()
            .map_err(|e: crate::asset::AssetError| VerificationError::InvalidSignature(e.to_string()))?;
        
        // 2. Verify network mode matches storage
        if asset.network != self.network_mode {
            return Err(VerificationError::NetworkMismatch);
        }
        
        // Mock successful storage
        Ok(())
    }
    
    fn network_mode(&self) -> NetworkMode {
        self.network_mode
    }
}

/// Memory-efficient streaming verification for large datasets
pub async fn stream_verify_assets(
    _storage: &dyn VerifiedStorage,
    limit: usize,
) -> Result<Vec<AssetEntry>, VerificationError> {
    let verified_assets = Vec::with_capacity(limit);
    
    // Mock implementation - in real usage this would query actual storage
    for _ in 0..limit {
        // Simulate asset verification
        
        // Memory check every 50 assets
        // Note: This is a mock implementation
        let memory_mb = get_memory_usage();
        if memory_mb > 400 { // Alert at 400MB to stay under 512MB limit
            tracing::warn!("High memory usage during verification: {} MB", memory_mb);
        }
    }
    
    Ok(verified_assets)
}

/// Get current memory usage in MB (Linux/Unix specific)
fn get_memory_usage() -> u64 {
    use std::fs;
    
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AssetMetadata, AssetType};
    use piva_crypto::{KeyPair, hash_blake3};
    
    #[tokio::test]
    async fn test_verified_storage_workflow() {
        let storage = MockVerifiedStorage::new(NetworkMode::Devnet);
        
        // Create test asset
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata {
            asset_type: AssetType::PropertyTitle,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890,
            description: "Test verified asset".to_string(),
            custom_fields: Default::default(),
        };
        
        let content = b"Test content for verification";
        let content_hash = hash_blake3(content);
        
        let asset = AssetEntry::new(
            metadata,
            content_hash,
            content.len() as u64,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        // Store with verification
        storage.store_asset_verified(&asset).unwrap();
        
        // Verify network mode
        assert_eq!(storage.network_mode(), NetworkMode::Devnet);
        
        println!("✅ Verified storage workflow test passed");
    }
    
    #[tokio::test]
    async fn test_corruption_detection() {
        let storage = MockVerifiedStorage::new(NetworkMode::Devnet);
        
        // Create asset with one keypair
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata {
            asset_type: AssetType::Diploma,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890,
            description: "Test corruption".to_string(),
            custom_fields: Default::default(),
        };
        
        let content = b"Test content";
        let content_hash = hash_blake3(content);
        
        let mut asset = AssetEntry::new(
            metadata,
            content_hash,
            content.len() as u64,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        // Store valid asset
        storage.store_asset_verified(&asset).unwrap();
        
        // Corrupt the signature (simulate bit rot)
        asset.signature[0] = asset.signature[0].wrapping_add(1);
        
        // Try to store corrupted asset (should fail)
        let result = storage.store_asset_verified(&asset);
        assert!(matches!(result, Err(VerificationError::InvalidSignature(_))));
        
        println!("✅ Corruption detection test passed");
    }
    
    #[tokio::test]
    async fn test_network_mode_isolation() {
        let devnet_storage = MockVerifiedStorage::new(NetworkMode::Devnet);
        
        // Create Devnet asset
        let keypair = KeyPair::generate();
        let metadata = AssetMetadata {
            asset_type: AssetType::LegalDocument,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890,
            description: "Cross-network test".to_string(),
            custom_fields: Default::default(),
        };
        
        let content = b"Test content";
        let content_hash = hash_blake3(content);
        
        let asset = AssetEntry::new(
            metadata,
            content_hash,
            content.len() as u64,
            NetworkMode::Testnet, // Wrong network mode!
            &keypair,
        ).unwrap();
        
        // Should fail network mode check
        let result = devnet_storage.store_asset_verified(&asset);
        assert!(matches!(result, Err(VerificationError::NetworkMismatch)));
        
        println!("✅ Network mode isolation test passed");
    }
}
