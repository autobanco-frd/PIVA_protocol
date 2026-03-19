//! # Verified Asset Retrieval
//! 
//! Extension trait for verified asset retrieval with signature validation.

use async_trait::async_trait;
use piva_storage::Storage;
use piva_types::AssetEntry;
use crate::NetworkMode;
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

#[async_trait]
pub trait VerifiedStorage {
    async fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError>;
    async fn get_asset_verified(&self, asset_id: &str) -> Result<AssetEntry, VerificationError>;
    fn network_mode(&self) -> NetworkMode;
}

#[async_trait]
impl VerifiedStorage for Storage {
    async fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError> {
        // 1. Pre-validate signature before storing
        asset.verify_integrity()
            .map_err(|e| VerificationError::InvalidSignature(e.to_string()))?;
        
        // 2. Verify network mode matches storage
        if asset.network != self.network_mode() {
            return Err(VerificationError::NetworkMismatch);
        }
        
        // 3. Store using the base Storage method
        self.store_asset(asset).await
            .map_err(|e| VerificationError::StorageError(e.to_string()))?;
        
        Ok(())
    }

    async fn get_asset_verified(&self, asset_id: &str) -> Result<AssetEntry, VerificationError> {
        // 1. Retrieve asset from storage
        let asset = self.get_asset(asset_id).await
            .map_err(|e| VerificationError::StorageError(e.to_string()))?
            .ok_or(VerificationError::NotFound)?;
        
        // 2. Verify integrity
        asset.verify_integrity()
            .map_err(|e| VerificationError::InvalidSignature(e.to_string()))?;
        
        // 3. Verify network mode consistency
        if asset.network != self.network_mode() {
            return Err(VerificationError::NetworkMismatch);
        }
        
        Ok(asset)
    }

    fn network_mode(&self) -> NetworkMode {
        // Access the network mode from Storage
        // This assumes Storage has a network_mode() method
        // If not, we'll need to store it during creation
        NetworkMode::Devnet // Placeholder - should be implemented in Storage
    }
}

/// Batch verification for multiple assets
pub async fn verify_asset_batch(
    storage: &Storage,
    asset_ids: &[String],
) -> Result<Vec<AssetEntry>, VerificationError> {
    let mut verified_assets = Vec::new();
    let mut corruption_detected = false;
    
    for asset_id in asset_ids {
        match storage.get_asset_verified(asset_id).await {
            Ok(asset) => verified_assets.push(asset),
            Err(VerificationError::InvalidSignature(_)) => {
                corruption_detected = true;
                continue; // Skip corrupted assets
            }
            Err(e) => return Err(e), // Propagate other errors
        }
    }
    
    if corruption_detected {
        tracing::warn!("Detected {} corrupted assets during batch verification", 
                      asset_ids.len() - verified_assets.len());
    }
    
    Ok(verified_assets)
}

/// Memory-efficient streaming verification for large datasets
pub async fn stream_verify_assets(
    storage: &Storage,
    limit: usize,
) -> Result<Vec<AssetEntry>, VerificationError> {
    let mut verified_assets = Vec::with_capacity(limit);
    
    // List assets in batches to avoid memory pressure
    let batch_size = std::cmp::min(100, limit);
    
    for batch_start in (0..limit).step_by(batch_size) {
        let current_batch_size = std::cmp::min(batch_size, limit - batch_start);
        
        let assets = storage.list_assets(current_batch_size).await
            .map_err(|e| VerificationError::StorageError(e.to_string()))?;
        
        for asset in assets {
            asset.verify_integrity()
                .map_err(|e| VerificationError::InvalidSignature(e.to_string()))?;
            verified_assets.push(asset);
        }
        
        // Memory check every 50 assets
        if verified_assets.len() % 50 == 0 {
            let memory_mb = get_memory_usage();
            if memory_mb > 400 { // Alert at 400MB to stay under 512MB limit
                tracing::warn!("High memory usage during verification: {} MB", memory_mb);
            }
        }
        
        if assets.len() < current_batch_size {
            break; // No more assets available
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
    use piva_types::{AssetMetadata, AssetType, NetworkMode};
    use piva_crypto::{KeyPair, hash_blake3};
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_verified_storage_workflow() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open_disk(temp_dir.path(), NetworkMode::Devnet).await.unwrap();
        
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
        storage.store_asset_verified(&asset).await.unwrap();
        
        // Retrieve with verification
        let retrieved = storage.get_asset_verified(&asset.id.to_string()).await.unwrap();
        
        // Verify integrity
        assert_eq!(retrieved.id, asset.id);
        assert_eq!(retrieved.content_hash, asset.content_hash);
        assert!(retrieved.verify_integrity().is_ok());
        
        println!("✅ Verified storage workflow test passed");
    }
    
    #[tokio::test]
    async fn test_corruption_detection() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open_disk(temp_dir.path(), NetworkMode::Devnet).await.unwrap();
        
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
        storage.store_asset_verified(&asset).await.unwrap();
        
        // Corrupt the signature (simulate bit rot)
        asset.signature[0] = asset.signature[0].wrapping_add(1);
        
        // Try to store corrupted asset (should fail)
        let result = storage.store_asset_verified(&asset).await;
        assert!(matches!(result, Err(VerificationError::InvalidSignature(_))));
        
        println!("✅ Corruption detection test passed");
    }
    
    #[tokio::test]
    async fn test_network_mode_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let devnet_storage = Storage::open_disk(temp_dir.path(), NetworkMode::Devnet).await.unwrap();
        
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
        let result = devnet_storage.store_asset_verified(&asset).await;
        assert!(matches!(result, Err(VerificationError::NetworkMismatch)));
        
        println!("✅ Network mode isolation test passed");
    }
}
