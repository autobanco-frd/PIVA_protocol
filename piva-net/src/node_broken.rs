//! # PIVA P2P Node
//! 
//! Main node implementation using Iroh for peer-to-peer networking.

use crate::config::NetworkConfig;
use bytes::Bytes;
use iroh::client::Iroh;
use piva_core::network::NetworkMode;
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

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
}

/// PIVA node wrapper around Iroh
pub struct PivaNode {
    client: Iroh,
    config: NetworkConfig,
}

impl PivaNode {
    /// Create a new PIVA node with the given configuration
    pub async fn new(config: NetworkConfig, _data_dir: PathBuf) -> Result<Self, NodeError> {
        info!("Starting PIVA node in {:?} mode", config.mode);
        
        // Create Iroh client - in 0.28 this is the main entry point
        let client = Iroh::new().await
            .map_err(|e| NodeError::IrohError(format!("Failed to create Iroh client: {}", e)))?;
        
        info!("PIVA node started with peer_id: {}", client.node_id());
        
        Ok(Self { client, config })
    }
    
    /// Get the node's configuration
    pub fn config(&self) -> &NetworkConfig {
        &self.config
    }
    
    /// Get the network mode
    pub fn network_mode(&self) -> NetworkMode {
        self.config.mode
    }
    
    /// Publish content to the P2P network
    pub async fn publish_content(&self, data: Bytes) -> Result<[u8; 32], NodeError> {
        info!("Publishing content of {} bytes", data.len());
        
        let outcome = self.client.blobs().add_bytes(data).await
            .map_err(|e| NodeError::ContentError(format!("Failed to add blob: {}", e)))?;
        
        // Extract hash bytes from AddOutcome
        let hash_bytes: [u8; 32] = *outcome.hash.as_bytes();
        
        info!("Content published with hash: {}", hex::encode(hash_bytes));
        Ok(hash_bytes)
    }
    
    /// Fetch content from the P2P network by hash
    pub async fn fetch_content(&self, hash: &[u8; 32]) -> Result<Bytes, NodeError> {
        info!("Fetching content with hash: {}", hex::encode(hash));
        
        use iroh::blobs::Hash;
        let hash = Hash::from_bytes(*hash);
        let data = self.client.blobs().get(hash).await
            .map_err(|e| NodeError::ContentError(format!("Failed to get blob: {}", e)))?
            .ok_or_else(|| NodeError::ContentError("Blob not found".to_string()))?;
        
        info!("Content fetched: {} bytes", data.len());
        Ok(data)
    }
    
    /// Verify a content chunk without downloading the full file
    pub async fn verify_chunk(
        &self,
        _hash: &[u8; 32],
        _offset: u64,
        _chunk: &[u8],
    ) -> Result<bool, NodeError> {
        // TODO: Implement BAO verified streaming
        Err(NodeError::ContentError("Not implemented yet".to_string()))
    }
    
    /// Get the node's peer ID
    pub fn peer_id(&self) -> String {
        self.client.node_id().to_string()
    }
    
    /// Start the node and begin listening for connections
    pub async fn start(&mut self) -> Result<(), NodeError> {
        info!("Node already started during creation");
        Ok(())
    }
    
    /// Stop the node gracefully
    pub async fn stop(&mut self) -> Result<(), NodeError> {
        info!("Stopping PIVA node");
        // Node is stopped when dropped
        Ok(())
    }
    
    /// Get current connection count
    pub fn connection_count(&self) -> usize {
        // TODO: Implement connection tracking
        0
    }
    
    /// Check if node is running
    pub fn is_running(&self) -> bool {
        true // Node is running after creation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::NetworkConfig;
    use piva_core::network::NetworkMode;
    
    #[tokio::test]
    async fn test_node_creation() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let node = PivaNode::new(config).await.unwrap();
        
        assert_eq!(node.network_mode(), NetworkMode::Devnet);
        assert_eq!(node.config.port, 7800);
        assert_eq!(node.config.max_connections, 5);
    }
    
    #[tokio::test]
    async fn test_node_lifecycle() {
        let config = NetworkConfig::new(NetworkMode::Devnet);
        let mut node = PivaNode::new(config).await.unwrap();
        
        assert!(!node.is_running());
        assert_eq!(node.connection_count(), 0);
        
        // Start node
        node.start().await.unwrap();
        assert!(node.is_running());
        
        // Stop node
        node.stop().await.unwrap();
        assert!(!node.is_running());
    }
}
