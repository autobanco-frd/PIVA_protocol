//! # Storage Layer
//! 
//! Main storage implementation using redb with async support.

use crate::tables::{ASSETS_TABLE, CONTENT_INDEX_TABLE};
use piva_core::{asset::{AssetEntry, AssetError}, network::NetworkMode};
use redb::{Database, ReadableTable};
use std::path::Path;
use thiserror::Error;
use tokio::task;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Asset not found")]
    NotFound,
    #[error("Invalid asset ID")]
    InvalidId,
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Internal error: {0}")]
    Internal(String),
}

// Implementar conversiones automáticas desde redb errors
impl From<redb::TransactionError> for StorageError {
    fn from(e: redb::TransactionError) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

impl From<AssetError> for StorageError {
    fn from(e: AssetError) -> Self {
        StorageError::SerializationError(e.to_string())
    }
}

impl From<redb::TableError> for StorageError {
    fn from(e: redb::TableError) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

impl From<redb::CommitError> for StorageError {
    fn from(e: redb::CommitError) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

impl From<redb::StorageError> for StorageError {
    fn from(e: redb::StorageError) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

impl From<redb::DatabaseError> for StorageError {
    fn from(e: redb::DatabaseError) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

impl From<tokio::task::JoinError> for StorageError {
    fn from(e: tokio::task::JoinError) -> Self {
        StorageError::Internal(e.to_string())
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::DatabaseError(e.to_string())
    }
}

/// Storage backend for PIVA assets
pub struct Storage {
    // Use Arc for thread-safe sharing across async tasks
    db: std::sync::Arc<redb::Database>,
    network: NetworkMode,
}

impl Storage {
    /// Open a disk-based database for Testnet/Mainnet
    pub async fn open_disk<P: AsRef<Path>>(path: P, network: NetworkMode) -> Result<Self, StorageError> {
        let path = path.as_ref().to_path_buf();
        let network_clone = network;
        
        let db = task::spawn_blocking(move || -> Result<redb::Database, StorageError> {
            std::fs::create_dir_all(&path)?;
            
            let db_path = path.join("piva.db");
            Database::create(db_path).map_err(Into::into)
        }).await??;
        
        Ok(Self { 
            db: std::sync::Arc::new(db), 
            network: network_clone 
        })
    }
    
    /// Open an in-memory database for Devnet
    pub async fn open_memory(network: NetworkMode) -> Result<Self, StorageError> {
        let db = task::spawn_blocking(|| -> Result<redb::Database, StorageError> {
            Database::builder()
                .create_with_backend(redb::backends::InMemoryBackend::new())
                .map_err(Into::into)
        }).await??;
        
        Ok(Self { 
            db: std::sync::Arc::new(db), 
            network 
        })
    }
    
    /// Store an asset entry
    pub async fn store_asset(&self, asset: &AssetEntry) -> Result<(), StorageError> {
        let asset_id = asset.id.to_string();
        let asset_bytes = asset.to_bytes()?;
        let content_hash = asset.content_hash;
        let content_hash_bytes = content_hash.as_slice().to_vec(); // Copy to owned
        
        let db = self.db.clone();
        
        task::spawn_blocking(move || -> Result<(), StorageError> {
            let write_txn = db.begin_write()?;
            
            {
                let mut assets_table = write_txn.open_table(ASSETS_TABLE)?;
                assets_table.insert(asset_id.as_bytes(), &*asset_bytes)?;
            }
            
            {
                let mut content_table = write_txn.open_table(CONTENT_INDEX_TABLE)?;
                content_table.insert(&content_hash_bytes[..], asset_id.as_bytes())?;
            }
            
            write_txn.commit()?;
            
            Ok(())
        }).await?
    }
    
    /// Retrieve an asset by ID
    pub async fn get_asset(&self, asset_id: &str) -> Result<Option<AssetEntry>, StorageError> {
        let db = self.db.clone();
        let id_string = asset_id.to_string();
        
        let result = task::spawn_blocking(move || -> Result<Option<AssetEntry>, StorageError> {
            let read_txn = db.begin_read()?;
            
            let assets_table = read_txn.open_table(ASSETS_TABLE)?;
            
            match assets_table.get(id_string.as_bytes())? {
                Some(value) => {
                    let asset_bytes = value.value();
                    let asset = AssetEntry::from_bytes(asset_bytes)?;
                    Ok(Some(asset))
                }
                None => Ok(None),
            }
        }).await?;
        
        result
    }
    
    /// List assets with optional limit
    pub async fn list_assets(&self, limit: usize) -> Result<Vec<AssetEntry>, StorageError> {
        let db = self.db.clone();
        
        let assets = task::spawn_blocking(move || -> Result<Vec<AssetEntry>, StorageError> {
            let read_txn = db.begin_read()?;
            
            let assets_table = read_txn.open_table(ASSETS_TABLE)?;
            
            let mut results = Vec::new();
            let mut count = 0;
            
            for item_result in assets_table.iter()? {
                if count >= limit {
                    break;
                }
                
                let (_key_guard, value_guard) = item_result?;
                let asset = AssetEntry::from_bytes(value_guard.value())
                    .map_err(|e: AssetError| StorageError::SerializationError(e.to_string()))?;
                
                results.push(asset);
                count += 1;
            }
            
            Ok(results)
        }).await?;
        
        assets
    }
    
    /// Delete an asset (only allowed in Devnet)
    pub async fn delete_asset(&self, asset_id: &str) -> Result<bool, StorageError> {
        if !matches!(self.network, NetworkMode::Devnet) {
            return Err(StorageError::PermissionDenied);
        }
        
        let db = self.db.clone();
        let id_string = asset_id.to_string();
        
        let deleted = task::spawn_blocking(move || -> Result<bool, StorageError> {
            let write_txn = db.begin_write()?;
            
            // Al usar .is_some() directamente, el AccessGuard se consume y muere
            // ANTES de que el scope termine.
            let removed = {
                let mut assets_table = write_txn.open_table(ASSETS_TABLE)?;
                let x = assets_table.remove(id_string.as_bytes())?.is_some();
                x
            }; 

            write_txn.commit()?; // Ahora write_txn es el único dueño y puede morir en paz
            Ok(removed)
        }).await.map_err(|e| StorageError::Internal(e.to_string()))??;
        
        Ok(deleted)
    }
    
    /// Get the network mode for this storage instance
    pub fn network(&self) -> NetworkMode {
        self.network
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piva_core::{asset::{AssetMetadata, AssetType}, network::NetworkMode};
    use piva_crypto::KeyPair;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_memory_storage() {
        let storage = Storage::open_memory(NetworkMode::Devnet).await.unwrap();
        assert_eq!(storage.network(), NetworkMode::Devnet);
    }
    
    #[tokio::test]
    async fn test_disk_storage() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::open_disk(temp_dir.path(), NetworkMode::Testnet).await.unwrap();
        assert_eq!(storage.network(), NetworkMode::Testnet);
    }
    
    #[tokio::test]
    async fn test_store_and_retrieve_asset() {
        let storage = Storage::open_memory(NetworkMode::Devnet).await.unwrap();
        let keypair = KeyPair::generate();
        
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair.public_key(),
            "Test Diploma".to_string(),
        );
        
        let asset = AssetEntry::new(
            metadata,
            [123u8; 32],
            1024,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        // Store asset
        storage.store_asset(&asset).await.unwrap();
        
        // Retrieve asset
        let retrieved = storage.get_asset(&asset.id.to_string()).await.unwrap();
        assert!(retrieved.is_some());
        
        let retrieved_asset = retrieved.unwrap();
        assert_eq!(retrieved_asset.id, asset.id);
        assert_eq!(retrieved_asset.content_hash, asset.content_hash);
        assert!(retrieved_asset.verify_integrity().is_ok());
    }
    
    #[tokio::test]
    async fn test_list_assets() {
        let storage = Storage::open_memory(NetworkMode::Devnet).await.unwrap();
        let keypair = KeyPair::generate();
        
        // Create multiple assets
        for i in 0..5 {
            let metadata = AssetMetadata::new(
                AssetType::Diploma,
                keypair.public_key(),
                format!("Test Diploma {}", i),
            );
            
            let asset = AssetEntry::new(
                metadata,
                [i; 32],
                1024,
                NetworkMode::Devnet,
                &keypair,
            ).unwrap();
            
            storage.store_asset(&asset).await.unwrap();
        }
        
        // List assets
        let assets = storage.list_assets(3).await.unwrap();
        assert_eq!(assets.len(), 3);
        
        let all_assets = storage.list_assets(10).await.unwrap();
        assert_eq!(all_assets.len(), 5);
    }
    
    #[tokio::test]
    async fn test_delete_asset_devnet_only() {
        let storage = Storage::open_memory(NetworkMode::Devnet).await.unwrap();
        let keypair = KeyPair::generate();
        
        let metadata = AssetMetadata::new(
            AssetType::Diploma,
            keypair.public_key(),
            "Test Diploma".to_string(),
        );
        
        let asset = AssetEntry::new(
            metadata,
            [123u8; 32],
            1024,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        storage.store_asset(&asset).await.unwrap();
        
        // Delete should work in Devnet
        let deleted = storage.delete_asset(&asset.id.to_string()).await.unwrap();
        assert!(deleted);
        
        // Verify it's gone
        let retrieved = storage.get_asset(&asset.id.to_string()).await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_delete_asset_mainnet_forbidden() {
        let storage = Storage::open_memory(NetworkMode::Mainnet).await.unwrap();
        
        // Delete should fail in Mainnet
        let result = storage.delete_asset("any_id").await;
        assert!(matches!(result, Err(StorageError::PermissionDenied)));
    }
}
