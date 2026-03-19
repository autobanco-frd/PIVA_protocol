//! # PIVA Core - Magic Bytes & Network Identity
//! 
//! Defines network isolation protocols and lineage-based identity system
//! for Sprint 5-6 implementation.

use piva_crypto::{hash_sha3_256, KeyPair};
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Invalid magic byte: {0:#x}")]
    InvalidMagicByte(u8),
    #[error("Network mode mismatch")]
    NetworkModeMismatch,
    #[error("Invalid lineage: {0}")]
    InvalidLineage(String),
}

/// Magic bytes for network isolation - Sprint 5.5
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicByte {
    Devnet = 0x01,
    Testnet = 0x02,
    Mainnet = 0x03,
}

impl MagicByte {
    pub fn from_network_mode(mode: crate::network::NetworkMode) -> Self {
        match mode {
            crate::network::NetworkMode::Devnet => MagicByte::Devnet,
            crate::network::NetworkMode::Testnet => MagicByte::Testnet,
            crate::network::NetworkMode::Mainnet => MagicByte::Mainnet,
        }
    }
    
    pub fn from_byte(byte: u8) -> Result<Self, NetworkError> {
        match byte {
            0x01 => Ok(MagicByte::Devnet),
            0x02 => Ok(MagicByte::Testnet),
            0x03 => Ok(MagicByte::Mainnet),
            _ => Err(NetworkError::InvalidMagicByte(byte)),
        }
    }
    
    pub fn as_byte(&self) -> u8 {
        *self as u8
    }
}

/// Lineage-based node identity - Sprint 6.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    /// Node's unique ID (SHA-3 of lineage + public key)
    pub node_id: [u8; 32],
    /// Parent node's ID (None for genesis nodes)
    pub parent_id: Option<[u8; 32]>,
    /// Node's public key
    pub public_key: [u8; 32],
    /// Network mode this node belongs to
    pub network_mode: crate::network::NetworkMode,
    /// Generation depth from genesis
    pub generation: u64,
    /// Creation timestamp
    pub created_at: u64,
}

impl NodeIdentity {
    /// Create new node identity with lineage - Sprint 6.1
    /// ID_Hijo = SHA3(ID_Padre + PublicKey_Hijo)
    pub fn new_child(
        parent_id: Option<[u8; 32]>,
        keypair: &KeyPair,
        network_mode: crate::network::NetworkMode,
    ) -> Result<Self, NetworkError> {
        let public_key = keypair.public_key();
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Calculate generation depth
        let generation = if parent_id.is_some() { 1 } else { 0 };
        
        // Compute node_id = SHA3(parent_id + public_key)
        let mut hash_input = Vec::new();
        if let Some(pid) = parent_id {
            hash_input.extend_from_slice(&pid);
        }
        hash_input.extend_from_slice(&public_key);
        
        let node_id = hash_sha3_256(&hash_input);
        
        Ok(Self {
            node_id,
            parent_id,
            public_key,
            network_mode,
            generation,
            created_at,
        })
    }
    
    /// Create genesis node (no parent)
    pub fn new_genesis(
        keypair: &KeyPair,
        network_mode: crate::network::NetworkMode,
    ) -> Result<Self, NetworkError> {
        Self::new_child(None, keypair, network_mode)
    }
    
    /// Verify lineage chain integrity
    pub fn verify_lineage(&self, parent_identity: &Option<NodeIdentity>) -> Result<bool, NetworkError> {
        match (&self.parent_id, parent_identity) {
            (None, None) => Ok(true), // Genesis node
            (Some(child_parent_id), Some(parent)) => {
                if child_parent_id != &parent.node_id {
                    return Ok(false);
                }
                
                // Recalculate this node's ID from parent
                let mut hash_input = Vec::new();
                hash_input.extend_from_slice(&parent.node_id);
                hash_input.extend_from_slice(&self.public_key);
                let expected_id = hash_sha3_256(&hash_input);
                
                Ok(self.node_id == expected_id)
            }
            _ => Ok(false), // Mismatch
        }
    }
    
    /// Get magic byte for this node's network
    pub fn magic_byte(&self) -> MagicByte {
        MagicByte::from_network_mode(self.network_mode)
    }
    
    /// Check if this node can connect to another based on network mode
    pub fn can_connect_to(&self, other: &NodeIdentity) -> bool {
        self.network_mode == other.network_mode
    }
}

/// Invitation package for node onboarding - Sprint 6.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationPackage {
    /// Parent node's ID
    pub parent_id: [u8; 32],
    /// Parent's multiaddr for Iroh connection
    pub parent_multiaddr: String,
    /// Network mode
    pub network_mode: crate::network::NetworkMode,
    /// Expected magic byte
    pub magic_byte: u8,
    /// Invitation expiration
    pub expires_at: u64,
    /// Parent's signature over invitation
    #[serde(with = "BigArray")]
    pub parent_signature: [u8; 64],
}

impl InvitationPackage {
    /// Create new invitation package
    pub fn new(
        parent_identity: &NodeIdentity,
        parent_multiaddr: String,
        parent_keypair: &KeyPair,
        ttl_hours: u64,
    ) -> Result<Self, NetworkError> {
        let expires_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() + (ttl_hours * 3600);
        
        let magic_byte = MagicByte::from_network_mode(parent_identity.network_mode);
        
        // Create invitation data to sign
        let invitation_data = format!(
            "{}:{}:{}:{}",
            hex::encode(parent_identity.node_id),
            parent_multiaddr,
            parent_identity.network_mode,
            expires_at
        );
        
        let parent_signature = parent_keypair.sign(invitation_data.as_bytes());
        let parent_signature_bytes: [u8; 64] = parent_signature.to_bytes();
        
        Ok(Self {
            parent_id: parent_identity.node_id,
            parent_multiaddr,
            network_mode: parent_identity.network_mode,
            magic_byte: magic_byte.as_byte(),
            expires_at,
            parent_signature: parent_signature_bytes,
        })
    }
    
    /// Verify invitation package signature
    pub fn verify(&self, parent_public_key: &[u8; 32]) -> Result<bool, NetworkError> {
        let invitation_data = format!(
            "{}:{}:{}:{}",
            hex::encode(self.parent_id),
            self.parent_multiaddr,
            self.network_mode,
            self.expires_at
        );
        
        KeyPair::verify(parent_public_key, invitation_data.as_bytes(), &self.parent_signature)
            .map_err(|_| NetworkError::InvalidLineage("Invalid signature".to_string()))
            .map(|_| true)
    }
    
    /// Check if invitation is still valid
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        now <= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_magic_byte_isolation() {
        let devnet_byte = MagicByte::from_network_mode(crate::network::NetworkMode::Devnet);
        let testnet_byte = MagicByte::from_network_mode(crate::network::NetworkMode::Testnet);
        
        assert_ne!(devnet_byte, testnet_byte);
        assert_eq!(devnet_byte.as_byte(), 0x01);
        assert_eq!(testnet_byte.as_byte(), 0x02);
        
        // Test invalid byte
        assert!(MagicByte::from_byte(0xFF).is_err());
    }
    
    #[test]
    fn test_node_identity_lineage() {
        let genesis_keypair = KeyPair::generate();
        let child_keypair = KeyPair::generate();
        
        let genesis = NodeIdentity::new_genesis(&genesis_keypair, crate::network::NetworkMode::Devnet)
            .unwrap();
        
        let child = NodeIdentity::new_child(
            Some(genesis.node_id),
            &child_keypair,
            crate::network::NetworkMode::Devnet,
        ).unwrap();
        
        // Verify lineage
        assert_eq!(child.parent_id, Some(genesis.node_id));
        assert_eq!(child.generation, 1);
        assert!(child.verify_lineage(&Some(genesis.clone())).unwrap());
        
        // Test network isolation
        assert!(child.can_connect_to(&genesis));
        assert!(!child.can_connect_to(&NodeIdentity::new_genesis(
            &KeyPair::generate(),
            crate::network::NetworkMode::Testnet
        ).unwrap()));
    }
    
    #[test]
    fn test_invitation_package() {
        let parent_keypair = KeyPair::generate();
        let parent = NodeIdentity::new_genesis(&parent_keypair, crate::network::NetworkMode::Testnet)
            .unwrap();
        
        let invitation = InvitationPackage::new(
            &parent,
            "/ip4/127.0.0.1/udp/7801/quic-v1".to_string(),
            &parent_keypair,
            24, // 24 hours TTL
        ).unwrap();
        
        // Verify invitation
        assert!(invitation.verify(&parent.public_key).unwrap());
        assert!(invitation.is_valid());
        assert_eq!(invitation.magic_byte, 0x02); // Testnet magic byte
    }
}
