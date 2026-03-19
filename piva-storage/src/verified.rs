//! # Verified Storage
//! 
//! Extension trait for verified asset retrieval with signature validation.

use async_trait::async_trait;
use crate::storage::Storage;
use piva_core::asset::AssetEntry;
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
}

#[async_trait]
pub trait VerifiedStorage {
    async fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError>;
    async fn get_asset_verified(&self, asset_id: &str) -> Result<AssetEntry, VerificationError>;
}

#[async_trait]
impl VerifiedStorage for Storage {
    async fn store_asset_verified(&self, asset: &AssetEntry) -> Result<(), VerificationError> {
        asset.verify_integrity()
            .map_err(|e| VerificationError::InvalidSignature(e.to_string()))?;

        if asset.network != self.network() {
            return Err(VerificationError::NetworkMismatch);
        }

        self.store_asset(asset).await
            .map_err(|e| VerificationError::StorageError(e.to_string()))?;

        Ok(())
    }

    async fn get_asset_verified(&self, asset_id: &str) -> Result<AssetEntry, VerificationError> {
        let asset = self.get_asset(asset_id).await
            .map_err(|e| VerificationError::StorageError(e.to_string()))?
            .ok_or(VerificationError::NotFound)?;

        asset.verify_integrity()
            .map_err(|e| VerificationError::InvalidSignature(e.to_string()))?;

        if asset.network != self.network() {
            return Err(VerificationError::NetworkMismatch);
        }

        Ok(asset)
    }
}
