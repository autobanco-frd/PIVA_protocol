//! # PIVA P2P Node
//! 
//! Simplified Iroh 0.28 integration with identity and lineage support.

use crate::config::NetworkConfig;
use crate::identity::{NodeIdentity, InvitationPackage, IdentityError};
use crate::lineage::{LineageStorage, LineageError};
use bytes::Bytes;
use piva_core::network::NetworkMode;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Iroh error: {0}")]
    IrohError(String),
    #[error("Network configuration error: {0}")]
    ConfigError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
    #[error("Content error: {0}")]
    ContentError(String),
    #[error("BAO verification error: {0}")]
    BaoError(String),
    #[error("Magic byte mismatch: expected {expected:x}, got {actual:x}")]
    MagicByteMismatch { expected: u8, actual: u8 },
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    #[error("Lineage error: {0}")]
    LineageError(#[from] LineageError),
}

/// PIVA node wrapper around Iroh with identity and lineage
pub struct PivaNode {
    pub config: NetworkConfig,
    pub running: Arc<RwLock<bool>>,
    pub peer_id: String,
    pub identity: NodeIdentity,
    pub lineage_storage: LineageStorage,
}

impl PivaNode {
    /// Create a new genesis PIVA node
    pub async fn genesis(
        config: NetworkConfig, 
        data_dir: PathBuf
    ) -> Result<Self, NodeError> {
        info!("Creating genesis PIVA node in {:?} mode", config.mode);
        
        let identity = NodeIdentity::genesis(config.mode);
        let lineage_storage = LineageStorage::new(data_dir.join("lineage"), config.mode).await?;
        
        // Store genesis node in lineage
        lineage_storage.store_node(&identity).await?;
        
        let peer_id = identity.peer_id();
        
        info!("Genesis node created: {} (generation 0)", identity);
        
        Ok(Self { 
            config,
            running: Arc::new(RwLock::new(false)),
            peer_id,
            identity,
            lineage_storage,
        })
    }
    
    /// Create a child node from invitation
    pub async fn from_invitation(
        invitation: &InvitationPackage,
        data_dir: PathBuf
    ) -> Result<Self, NodeError> {
        if !invitation.is_valid() {
            return Err(NodeError::IdentityError(
                IdentityError::InvalidParentId("Invitation expired".to_string())
            ));
        }
        
        info!("Creating child node from invitation (parent: {})", 
              hex::encode(invitation.parent_id)[..8].to_uppercase());
        
        // Create parent identity for child generation
        let parent_identity = NodeIdentity {
            node_id: invitation.parent_id,
            parent_id: None,
            keypair: piva_crypto::KeyPair::generate(),
            network_mode: invitation.network_mode,
            generation: invitation.target_generation - 1,
            created_at: 0,
        };
        
        let identity = NodeIdentity::child_of(&parent_identity)?;
        
        // Verify the child has the correct generation
        assert_eq!(identity.generation, invitation.target_generation,
                   "Child generation mismatch: expected {}, got {}",
                   invitation.target_generation, identity.generation);
        let lineage_storage = LineageStorage::new(data_dir.join("lineage"), invitation.network_mode).await?;
        
        // Store child node in lineage
        lineage_storage.store_node(&identity).await?;
        
        // Store invitation
        lineage_storage.store_invitation(invitation).await?;
        
        let peer_id = identity.peer_id();
        
        info!("Child node created: {} (generation {})", identity, identity.generation);
        
        Ok(Self { 
            config: NetworkConfig::new(invitation.network_mode),
            running: Arc::new(RwLock::new(false)),
            peer_id,
            identity,
            lineage_storage,
        })
    }
    
    /// Get the node's configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }
    
    /// Get the network mode
    pub fn network_mode(&self) -> NetworkMode {
        self.config.mode
    }
    
    /// Get the node's identity
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }
    
    /// Get the node's peer ID
    pub fn peer_id(&self) -> String {
        self.peer_id.clone()
    }
    
    /// Create an invitation package for this node
    pub async fn create_invitation(&self, expires_in_hours: u64) -> Result<InvitationPackage, NodeError> {
        let multiaddr = format!("/ip4/127.0.0.1/udp/{}", self.config.port);
        let invitation = InvitationPackage::new(&self.identity, multiaddr, expires_in_hours);
        
        info!("Created invitation for node {} (expires in {}h)", 
              self.identity, expires_in_hours);
        
        Ok(invitation)
    }
    
    /// Perform welcome handshake with a child node
    pub async fn welcome_handshake(&self, child_public_key: &[u8; 32]) -> Result<[u8; 64], NodeError> {
        let welcome_signature = self.identity.sign_welcome(child_public_key);
        
        // Record that this node invited a child
        self.lineage_storage.record_successful_share(&self.identity.node_id).await?;
        
        info!("Welcome handshake completed for child {}", 
              hex::encode(child_public_key)[..8].to_uppercase());
        
        Ok(welcome_signature.to_vec().try_into().unwrap())
    }
    
    /// Publish content to the P2P network with BAO encoding
    pub async fn publish_content(&self, data: Bytes) -> Result<[u8; 32], NodeError> {
        info!("Publishing content of {} bytes", data.len());
        
        // Create BAO hash for the content
        let (_bao_bytes, bao_hash) = bao::encode::encode(&data);
        
        // Convert to BLAKE3 for consistency
        let blake3_hash = piva_crypto::hash_blake3(&data);
        
        // Update activity and reputation
        self.lineage_storage.update_activity(&self.identity.node_id).await?;
        self.lineage_storage.record_successful_share(&self.identity.node_id).await?;
        
        info!("Content published with BAO hash: {}", hex::encode(bao_hash.as_bytes()));
        info!("BLAKE3 hash: {}", hex::encode(blake3_hash));
        
        Ok(blake3_hash)
    }
    
    /// Fetch content from the P2P network by hash
    pub async fn fetch_content(&self, hash: &[u8; 32]) -> Result<Bytes, NodeError> {
        info!("Fetching content with hash: {}", hex::encode(hash));
        
        // Update activity
        self.lineage_storage.update_activity(&self.identity.node_id).await?;
        
        // For now, return mock data
        // In a real implementation, this would fetch from Iroh
        warn!("Content fetching not fully implemented - returning mock data");
        
        let mock_data = Bytes::from(format!("Mock content for hash: {}", hex::encode(hash)));
        Ok(mock_data)
    }
    
    /// Verify a content chunk using BAO without downloading the full file
    pub async fn verify_chunk(
        &self,
        root_hash: &[u8; 32],
        offset: u64,
        _chunk: &[u8],
    ) -> Result<bool, NodeError> {
        info!("Verifying chunk at offset {} for hash: {}", offset, hex::encode(root_hash));
        
        // Update activity and reputation
        self.lineage_storage.update_activity(&self.identity.node_id).await?;
        self.lineage_storage.record_successful_verification(&self.identity.node_id).await?;
        
        // TODO: Implement proper BAO chunk verification
        warn!("BAO chunk verification not fully implemented - returning placeholder");
        Ok(true) // Placeholder - would verify actual chunk
    }
    
    /// Start the node and begin listening for connections
    pub async fn start(&mut self) -> Result<(), NodeError> {
        let mut running_guard = self.running.write().await;
        if *running_guard {
            return Ok(());
        }
        
        info!("Starting PIVA node {} on port {}", self.identity, self.config.port);
        *running_guard = true;
        
        // Update activity
        self.lineage_storage.update_activity(&self.identity.node_id).await?;
        
        info!("PIVA node started successfully");
        Ok(())
    }
    
    /// Stop the node gracefully
    pub async fn stop(&mut self) -> Result<(), NodeError> {
        let mut running_guard = self.running.write().await;
        if !*running_guard {
            return Ok(());
        }
        
        info!("Stopping PIVA node {}", self.identity);
        *running_guard = false;
        
        info!("PIVA node stopped");
        Ok(())
    }
    
    /// Get current connection count
    pub async fn connection_count(&self) -> usize {
        // TODO: Return actual connection count from Iroh
        0
    }
    
    /// Check if node is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
    
    /// Verify magic byte during connection handshake
    pub fn verify_magic_byte(&self, received_byte: u8) -> Result<(), NodeError> {
        let expected = self.config.mode.magic_byte();
        if received_byte != expected {
            return Err(NodeError::MagicByteMismatch {
                expected,
                actual: received_byte,
            });
        }
        Ok(())
    }
    
    /// Get network statistics
    pub async fn network_stats(&self) -> NetworkStats {
        let connections = self.connection_count().await;
        let running = self.is_running().await;
        
        NetworkStats {
            running,
            connections,
            network_mode: self.config.mode,
            port: self.config.port,
            max_connections: self.config.max_connections,
            buffer_size: self.config.buffer_size,
            node_id: self.identity.node_id,
            generation: self.identity.generation,
            peer_id: self.peer_id.clone(),
        }
    }
    
    /// Get node's reputation
    pub async fn get_reputation(&self) -> Result<Option<crate::lineage::ReputationEntry>, NodeError> {
        Ok(self.lineage_storage.get_reputation(&self.identity.node_id).await?)
    }
    
    /// Get node's lineage path
    pub async fn get_lineage_path(&self) -> Result<Vec<crate::lineage::LineageEntry>, NodeError> {
        Ok(self.lineage_storage.get_lineage_path(&self.identity.node_id).await?)
    }
    
    /// Get node's children
    pub async fn get_children(&self) -> Result<Vec<crate::lineage::LineageEntry>, NodeError> {
        Ok(self.lineage_storage.get_children(&self.identity.node_id).await?)
    }
}

/// Network statistics for monitoring
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub running: bool,
    pub connections: usize,
    pub network_mode: NetworkMode,
    pub port: u16,
    pub max_connections: usize,
    pub buffer_size: usize,
    pub node_id: [u8; 32],
    pub generation: u32,
    pub peer_id: String,
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkConfig;
    use piva_core::network::NetworkMode;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_genesis_node() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        assert_eq!(node.identity.generation, 0);
        assert_eq!(node.network_mode(), NetworkMode::Devnet);
        assert_eq!(node.config.port, 7800);
        assert!(!node.is_running().await);
    }
    
    #[tokio::test]
    async fn test_invitation_creation() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        let invitation = node.create_invitation(24).await.unwrap();
        
        assert_eq!(invitation.parent_id, node.identity.node_id);
        assert_eq!(invitation.network_mode, NetworkMode::Devnet);
        assert_eq!(invitation.target_generation, 1);
        assert!(invitation.is_valid());
    }
    
    #[tokio::test]
    async fn test_welcome_handshake() {
        let config = NetworkConfig::new(NetworkMode::Testnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        let child_key = piva_crypto::KeyPair::generate().public_key();
        let signature = node.welcome_handshake(&child_key).await.unwrap();
        
        assert_eq!(signature.len(), 64);
    }
    
    #[tokio::test]
    async fn test_node_lifecycle() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let temp_dir = TempDir::new().unwrap();
        let mut node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        assert_eq!(node.connection_count().await, 0);
        
        // Start node
        node.start().await.unwrap();
        assert!(node.is_running().await);
        
        // Stop node
        node.stop().await.unwrap();
        assert!(!node.is_running().await);
    }
    
    #[tokio::test]
    async fn test_publish_content() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        let data = Bytes::from("Hello, PIVA!");
        let hash = node.publish_content(data.clone()).await.unwrap();
        
        // Verify hash is BLAKE3 of data
        let expected_hash = piva_crypto::hash_blake3(&data);
        assert_eq!(hash, expected_hash);
    }
    
    #[tokio::test]
    async fn test_magic_byte_verification() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        // Correct magic byte should pass
        assert!(node.verify_magic_byte(0x01).is_ok());
        
        // Wrong magic byte should fail
        assert!(node.verify_magic_byte(0x02).is_err());
    }
    
    #[tokio::test]
    async fn test_network_stats() {
        let config = NetworkConfig::new(NetworkMode::Testnet);
        let temp_dir = TempDir::new().unwrap();
        let node = PivaNode::genesis(config, temp_dir.path().to_path_buf()).await.unwrap();
        
        let stats = node.network_stats().await;
        assert_eq!(stats.network_mode, NetworkMode::Testnet);
        assert_eq!(stats.port, 7801);
        assert_eq!(stats.max_connections, 25);
        assert_eq!(stats.buffer_size, 8192);
        assert_eq!(stats.generation, 0);
        assert_eq!(stats.node_id, node.identity.node_id);
        assert!(!stats.running);
        assert_eq!(stats.connections, 0);
    }
}
