//! # RWA and Multimedia Management
//!
//! Real World Assets (RWA) handling with verified streaming, marketplace, and ISO 20022 support.

use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;
use thiserror::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use piva_crypto::{hash, verify_signature, create_signature};

// Include marketplace and ISO 20022 modules
pub mod market;
pub mod iso20022;
pub mod matching;
pub mod scoring;
pub mod multisig;

// Re-export marketplace types
pub use market::{
    MarketOffer, OfferStatus, OfferType, PeerScore
};

// Re-export ISO 20022 types
pub use iso20022::{
    Iso20022Report, IsoMessageType, TransferStatus
};

// Re-export matching engine types
pub use matching::{
    MatchingEngine, OrderBook, Order, TradeMatch, OrderStatus as MatchingOrderStatus,
    MatchingConfig, EngineStats, OrderBookStats
};

// Re-export advanced scoring types
pub use scoring::{
    AdvancedScoringEngine, PeerData, TradeRecord, TradeOutcome,
    ScoringConfig, GlobalScoringStats, ScoreDistribution
};

// Re-export multisig types
pub use multisig::{
    MultiSigManager, MultiSigWallet, SignerInfo, SignerRole,
    PendingTransaction, CompletedTransaction, TransactionType,
    WalletState, SecurityLevel, MultiSigConfig
};

#[derive(Error, Debug)]
pub enum RwaError {
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Invalid RWA format: {0}")]
    InvalidFormat(String),
    #[error("Revocation error: {0}")]
    RevocationError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// RWA Asset Types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RwaAssetType {
    /// Academic titles and diplomas
    AcademicTitle {
        institution: String,
        degree: String,
        issue_date: u64,
        graduate_id: String,
    },
    /// Professional certifications
    Certification {
        issuer: String,
        certification_name: String,
        expiration_date: Option<u64>,
        certificate_id: String,
    },
    /// Audio content with metadata
    AudioContent {
        title: String,
        artist: String,
        duration_seconds: u32,
        bitrate: u32,
        format: AudioFormat,
    },
    /// Video content
    VideoContent {
        title: String,
        duration_seconds: u32,
        resolution: String,
        format: VideoFormat,
    },
}

/// Audio formats supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    OggVorbis,
    Flac,
    Aac,
    Opus,
}

/// Video formats supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VideoFormat {
    Mp4,
    WebM,
    Avi,
    Mov,
}

/// RWA Asset with verification metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RwaAsset {
    /// Unique asset identifier
    pub asset_id: [u8; 32],
    /// Asset type and specific metadata
    pub asset_type: RwaAssetType,
    /// Content hash for integrity verification
    pub content_hash: [u8; 32],
    /// Issuer signature
    #[serde(with = "BigArray")]
    pub issuer_signature: [u8; 64],
    /// Creation timestamp
    pub created_at: u64,
    /// Revocation status
    pub is_revoked: bool,
    /// Revocation reason (if revoked)
    pub revocation_reason: Option<String>,
    /// Revocation timestamp (if revoked) - for audit consistency
    pub revoked_at: Option<u64>,
    /// Verification metadata
    pub verification_metadata: VerificationMetadata,
}

/// Verification metadata for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMetadata {
    /// BAO tree root for verified streaming
    pub bao_root: [u8; 32],
    /// Total size in bytes
    pub total_size: u64,
    /// Chunk size for streaming
    pub chunk_size: u32,
    /// Number of chunks
    pub chunk_count: u32,
    /// Merkle tree depth
    pub tree_depth: u8,
}

/// Chunk for verified streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedChunk {
    /// Chunk index
    pub index: u32,
    /// Chunk data
    pub data: Vec<u8>,
    /// BAO proof for this chunk
    pub bao_proof: Vec<u8>,
    /// Chunk hash
    pub hash: [u8; 32],
    /// Timestamp when chunk was verified
    pub verified_at: u64,
}

/// Revocation certificate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevocationCertificate {
    /// Asset being revoked
    pub asset_id: [u8; 32],
    /// Revocation reason
    pub reason: String,
    /// Issuer signature for revocation
    #[serde(with = "BigArray")]
    pub issuer_signature: [u8; 64],
    /// Revocation timestamp
    pub revoked_at: u64,
}

impl RwaAsset {
    /// Create new RWA asset
    pub fn new(
        asset_type: RwaAssetType,
        content: &[u8],
        issuer_private_key: &[u8; 32],
    ) -> Result<Self, RwaError> {
        let asset_id = hash(content);
        let content_hash = hash(content);
        
        // Create BAO tree for verified streaming
        let bao_root = Self::create_bao_tree(content)?;
        let chunk_size = Self::optimal_chunk_size(content.len(), &asset_type);
        let chunk_count = (content.len() as u32 + chunk_size - 1) / chunk_size;
        
        let verification_metadata = VerificationMetadata {
            bao_root,
            total_size: content.len() as u64,
            chunk_size,
            chunk_count,
            tree_depth: Self::calculate_tree_depth(chunk_count),
        };
        
        // Create issuer signature
        let signature_data = Self::create_signature_data(&asset_id, &content_hash, &verification_metadata);
        let issuer_signature = create_signature(&signature_data, issuer_private_key)
            .map_err(|e: Box<dyn std::error::Error>| RwaError::VerificationFailed(e.to_string()))?;
        
        Ok(Self {
            asset_id,
            asset_type,
            content_hash,
            issuer_signature,
            created_at: now_secs(),
            is_revoked: false,
            revocation_reason: None,
            revoked_at: None,
            verification_metadata,
        })
    }
    
    /// Verify asset integrity and authenticity
    pub fn verify(&self, issuer_public_key: &[u8; 32]) -> Result<bool, RwaError> {
        // Check if revoked
        if self.is_revoked {
            return Ok(false);
        }
        
        // Verify issuer signature
        let signature_data = Self::create_signature_data(&self.asset_id, &self.content_hash, &self.verification_metadata);
        verify_signature(&signature_data, &self.issuer_signature, issuer_public_key)
            .map_err(|e: Box<dyn std::error::Error>| RwaError::VerificationFailed(e.to_string()))?;
        
        Ok(true)
    }
    
    /// Create verified chunk for streaming
    pub fn create_chunk(&self, content: &[u8], index: u32) -> Result<VerifiedChunk, RwaError> {
        if index >= self.verification_metadata.chunk_count {
            return Err(RwaError::InvalidFormat("Chunk index out of bounds".to_string()));
        }
        
        let start = (index as usize) * (self.verification_metadata.chunk_size as usize);
        let end = std::cmp::min(start + (self.verification_metadata.chunk_size as usize), content.len());
        
        if start >= content.len() {
            return Err(RwaError::InvalidFormat("Invalid chunk range".to_string()));
        }
        
        let chunk_data = content[start..end].to_vec();
        let chunk_hash = hash(&chunk_data);
        
        // Create BAO proof for this chunk
        let bao_proof = Self::create_bao_proof(&chunk_data, index, &self.verification_metadata)?;
        
        Ok(VerifiedChunk {
            index,
            data: chunk_data,
            bao_proof,
            hash: chunk_hash,
            verified_at: now_secs(),
        })
    }
    
    /// Verify chunk integrity using BAO proof
    pub fn verify_chunk(&self, chunk: &VerifiedChunk) -> Result<bool, RwaError> {
        // Verify chunk hash
        let computed_hash = hash(&chunk.data);
        if computed_hash != chunk.hash {
            return Ok(false);
        }
        
        // Verify BAO proof against the root
        let is_valid = self.verify_bao_proof(&chunk.data, chunk.index, &chunk.bao_proof)?;
        
        Ok(is_valid)
    }
    
    /// Revoke asset with timestamp consistency for audit
    pub fn revoke(&mut self, reason: String, issuer_private_key: &[u8; 32]) -> Result<RevocationCertificate, RwaError> {
        let timestamp = now_secs(); // Fixed timestamp for this revocation
        let revocation_data = self.create_revocation_data_with_time(&reason, timestamp);
        
        let issuer_signature = create_signature(&revocation_data, issuer_private_key)
            .map_err(|e: Box<dyn std::error::Error>| RwaError::RevocationError(e.to_string()))?;
        
        let certificate = RevocationCertificate {
            asset_id: self.asset_id,
            reason: reason.clone(),
            issuer_signature,
            revoked_at: timestamp,
        };
        
        self.is_revoked = true;
        self.revocation_reason = Some(reason);
        self.revoked_at = Some(timestamp); // Store timestamp for audit consistency
        
        Ok(certificate)
    }

    /// Verify revocation certificate authenticity
    pub fn verify_revocation(&self, certificate: &RevocationCertificate, issuer_public_key: &[u8; 32]) -> Result<bool, RwaError> {
        // Check if certificate matches this asset
        if certificate.asset_id != self.asset_id {
            return Ok(false);
        }
        
        // Check if timestamps match
        if let Some(revoked_at) = self.revoked_at {
            if certificate.revoked_at != revoked_at {
                return Ok(false);
            }
        } else {
            return Ok(false); // Asset not revoked
        }
        
        // Verify certificate signature
        let revocation_data = self.create_revocation_data_with_time(&certificate.reason, certificate.revoked_at);
        verify_signature(&revocation_data, &certificate.issuer_signature, issuer_public_key)
            .map_err(|e: Box<dyn std::error::Error>| RwaError::RevocationError(e.to_string()))?;
        
        Ok(true)
    }
    
    /// Get optimal chunk size based on asset type and content size
    fn optimal_chunk_size(content_size: usize, asset_type: &RwaAssetType) -> u32 {
        match asset_type {
            RwaAssetType::AudioContent { .. } => {
                // 64KB chunks for mobile devices with limited RAM
                if content_size < 1024 * 1024 { // < 1MB
                    32 * 1024 // 32KB
                } else if content_size < 10 * 1024 * 1024 { // < 10MB
                    64 * 1024 // 64KB
                } else {
                    128 * 1024 // 128KB
                }
            }
            RwaAssetType::VideoContent { .. } => {
                // Larger chunks for video
                if content_size < 10 * 1024 * 1024 { // < 10MB
                    128 * 1024 // 128KB
                } else if content_size < 100 * 1024 * 1024 { // < 100MB
                    256 * 1024 // 256KB
                } else {
                    512 * 1024 // 512KB
                }
            }
            _ => {
                // Default chunk size for documents
                64 * 1024 // 64KB
            }
        }
    }
    
    /// Create BAO tree root (simplified implementation)
    fn create_bao_tree(content: &[u8]) -> Result<[u8; 32], RwaError> {
        // In a real implementation, this would use the actual BAO algorithm
        // For now, we'll use a simple hash as a placeholder
        Ok(hash(content))
    }
    
    /// Create BAO proof for a chunk (simplified implementation)
    fn create_bao_proof(_chunk_data: &[u8], _index: u32, _metadata: &VerificationMetadata) -> Result<Vec<u8>, RwaError> {
        // In a real implementation, this would generate the actual BAO proof
        // For now, we'll use a placeholder
        Ok(vec![1, 2, 3, 4]) // Placeholder proof
    }
    
    /// Verify BAO proof (simplified implementation)
    fn verify_bao_proof(&self, _chunk_data: &[u8], _index: u32, _proof: &[u8]) -> Result<bool, RwaError> {
        // In a real implementation, this would verify the actual BAO proof
        // For now, we'll always return true as a placeholder
        Ok(true)
    }
    
    /// Calculate Merkle tree depth
    fn calculate_tree_depth(chunk_count: u32) -> u8 {
        if chunk_count <= 1 {
            0
        } else {
            (chunk_count as f64).log2().ceil() as u8
        }
    }
    
    /// Create signature data for verification
    fn create_signature_data(asset_id: &[u8; 32], content_hash: &[u8; 32], metadata: &VerificationMetadata) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(asset_id);
        data.extend_from_slice(content_hash);
        data.extend_from_slice(&metadata.bao_root);
        data.extend_from_slice(&metadata.total_size.to_le_bytes());
        data.extend_from_slice(&metadata.chunk_size.to_le_bytes());
        data
    }
    
    /// Create revocation data with explicit timestamp
    fn create_revocation_data_with_time(&self, reason: &str, timestamp: u64) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.asset_id);
        data.extend_from_slice(reason.as_bytes());
        data.extend_from_slice(&timestamp.to_le_bytes()); // Deterministic timestamp
        data
    }
}

/// Get current timestamp
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use piva_crypto::SigningKey;
    use rand::thread_rng;

    #[test]
    fn test_rwa_asset_creation() {
        let asset_type = RwaAssetType::AudioContent {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 180,
            bitrate: 320,
            format: AudioFormat::Mp3,
        };
        
        let content = b"This is test audio content for RWA asset";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        let issuer_public_key = signing_key.verifying_key().to_bytes();
        
        let asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        assert_eq!(asset.content_hash, hash(content));
        assert!(!asset.is_revoked);
        assert!(asset.created_at > 0);
        assert!(asset.revoked_at.is_none());
        
        // Verify asset with derived public key
        let is_valid = asset.verify(&issuer_public_key).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_chunk_creation() {
        let asset_type = RwaAssetType::AudioContent {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 180,
            bitrate: 320,
            format: AudioFormat::Mp3,
        };
        
        let content = b"This is test audio content for RWA asset that is longer than 64 bytes to test chunking properly";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        
        let asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        // Create first chunk
        let chunk = asset.create_chunk(content, 0).unwrap();
        assert_eq!(chunk.index, 0);
        assert!(!chunk.data.is_empty());
        
        // Verify chunk
        let is_valid = asset.verify_chunk(&chunk).unwrap();
        assert!(is_valid);
    }

    #[test]
    fn test_asset_revocation() {
        let asset_type = RwaAssetType::Certification {
            issuer: "Test University".to_string(),
            certification_name: "Test Certificate".to_string(),
            expiration_date: Some(now_secs() + 365 * 24 * 3600),
            certificate_id: "CERT-001".to_string(),
        };
        
        let content = b"Test certificate content";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        let issuer_public_key = signing_key.verifying_key().to_bytes();
        
        let mut asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        // Revoke asset
        let certificate = asset.revoke("Test revocation".to_string(), &issuer_private_key).unwrap();
        
        assert!(asset.is_revoked);
        assert_eq!(asset.revocation_reason, Some("Test revocation".to_string()));
        assert!(asset.revoked_at.is_some());
        assert_eq!(certificate.asset_id, asset.asset_id);
        assert_eq!(certificate.revoked_at, asset.revoked_at.unwrap());
        
        // Verify revocation certificate
        let is_revocation_valid = asset.verify_revocation(&certificate, &issuer_public_key).unwrap();
        assert!(is_revocation_valid);
        
        // Asset should no longer be valid
        let is_asset_valid = asset.verify(&issuer_public_key).unwrap();
        assert!(!is_asset_valid);
    }

    // Negative Path Tests
    #[test]
    fn test_chunk_corruption() {
        let asset_type = RwaAssetType::AudioContent {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 180,
            bitrate: 320,
            format: AudioFormat::Mp3,
        };
        
        let content = b"This is test audio content for corruption testing";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        
        let asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        // Create valid chunk
        let mut chunk = asset.create_chunk(content, 0).unwrap();
        
        // Corrupt chunk data
        chunk.data[0] ^= 0xFF; // Flip some bits
        
        // Verification should fail
        let is_valid = asset.verify_chunk(&chunk).unwrap();
        assert!(!is_valid);
    }

    #[test]
    fn test_chunk_out_of_bounds() {
        let asset_type = RwaAssetType::AudioContent {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 180,
            bitrate: 320,
            format: AudioFormat::Mp3,
        };
        
        let content = b"Short content";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        
        let asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        // Try to create chunk with index out of bounds
        let result = asset.create_chunk(content, 999);
        assert!(result.is_err());
        
        match result.unwrap_err() {
            RwaError::InvalidFormat(msg) => assert!(msg.contains("out of bounds")),
            _ => panic!("Expected InvalidFormat error"),
        }
    }

    #[test]
    fn test_invalid_signature_verification() {
        let asset_type = RwaAssetType::AudioContent {
            title: "Test Song".to_string(),
            artist: "Test Artist".to_string(),
            duration_seconds: 180,
            bitrate: 320,
            format: AudioFormat::Mp3,
        };
        
        let content = b"Test content";
        let signing_key = SigningKey::generate(&mut thread_rng());
        let issuer_private_key = signing_key.to_bytes();
        let wrong_public_key = [99u8; 32]; // Wrong public key
        
        let asset = RwaAsset::new(asset_type, content, &issuer_private_key).unwrap();
        
        // Verification should fail with wrong public key
        let is_valid = asset.verify(&wrong_public_key);
        assert!(is_valid.is_err());
        
        match is_valid.unwrap_err() {
            RwaError::VerificationFailed(_) => {}, // Expected
            _ => panic!("Expected VerificationFailed error"),
        }
    }
}
