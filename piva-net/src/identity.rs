//! # PIVA Node Identity with Lineage
//!
//! Hierarchical node identity system using piva-crypto primitives.

use piva_crypto::{KeyPair, hash_sha3_256};
use piva_core::network::NetworkMode;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde_bytes::ByteArray;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Invalid parent ID: {0}")]
    InvalidParentId(String),
    #[error("KeyPair error: {0}")]
    KeyPairError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid lineage depth: {0}")]
    InvalidDepth(u32),
}

/// Node identity with hierarchical lineage.
/// Uses manual Serde impl because KeyPair doesn't derive Serialize/Deserialize.
#[derive(Debug, Clone)]
pub struct NodeIdentity {
    pub node_id: [u8; 32],
    pub parent_id: Option<[u8; 32]>,
    pub keypair: KeyPair,
    pub network_mode: NetworkMode,
    pub generation: u32,
    pub created_at: u64,
}

// --------------- Serde via secret_key bytes ---------------

impl Serialize for NodeIdentity {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("NodeIdentity", 6)?;
        st.serialize_field("node_id", &self.node_id)?;
        st.serialize_field("parent_id", &self.parent_id)?;
        st.serialize_field("network_mode", &self.network_mode)?;
        st.serialize_field("generation", &self.generation)?;
        st.serialize_field("created_at", &self.created_at)?;
        // Use serde_bytes to handle [u8; 64] serialization
        let bytes = self.keypair.to_bytes();
        st.serialize_field("keypair_bytes", serde_bytes::Bytes::new(&bytes))?;
        st.end()
    }
}

impl<'de> Deserialize<'de> for NodeIdentity {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct H {
            node_id: [u8; 32],
            parent_id: Option<[u8; 32]>,
            network_mode: NetworkMode,
            generation: u32,
            created_at: u64,
            #[serde(with = "serde_bytes")]
            keypair_bytes: [u8; 64],
        }
        let h = H::deserialize(d)?;
        let keypair = KeyPair::from_bytes(&h.keypair_bytes)
            .map_err(|e| serde::de::Error::custom(e.to_string()))?;
        Ok(Self {
            node_id: h.node_id,
            parent_id: h.parent_id,
            keypair,
            network_mode: h.network_mode,
            generation: h.generation,
            created_at: h.created_at,
        })
    }
}

// --------------- Core logic ---------------

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl NodeIdentity {
    /// Create a genesis node (root of lineage tree).
    pub fn genesis(network_mode: NetworkMode) -> Self {
        let keypair = KeyPair::generate();
        let seed = format!("{:?}", network_mode);
        let node_id = hash_sha3_256(&[seed.as_bytes(), &keypair.public_key()].concat());
        Self {
            node_id,
            parent_id: None,
            keypair,
            network_mode,
            generation: 0,
            created_at: now_secs(),
        }
    }

    /// Create a child node derived from a parent.
    pub fn child_of(parent: &NodeIdentity) -> Result<Self, IdentityError> {
        if parent.generation >= 100 {
            return Err(IdentityError::InvalidDepth(parent.generation + 1));
        }
        let keypair = KeyPair::generate();
        let node_id = hash_sha3_256(&[&parent.node_id[..], &keypair.public_key()].concat());
        Ok(Self {
            node_id,
            parent_id: Some(parent.node_id),
            keypair,
            network_mode: parent.network_mode,
            generation: parent.generation + 1,
            created_at: now_secs(),
        })
    }

    pub fn public_key(&self) -> [u8; 32] {
        self.keypair.public_key()
    }

    pub fn peer_id(&self) -> String {
        hex::encode(self.node_id)
    }

    /// Sign arbitrary data.
    pub fn sign(&self, data: &[u8]) -> [u8; 64] {
        let sig = self.keypair.sign(data);
        sig.to_bytes()
    }

    /// Verify a signature using this node's public key.
    pub fn verify_signature(&self, data: &[u8], signature: &[u8; 64]) -> Result<(), IdentityError> {
        KeyPair::verify(&self.public_key(), data, signature)
            .map_err(|e| IdentityError::KeyPairError(e.to_string()))
    }

    /// Create a welcome signature for a child node.
    pub fn sign_welcome(&self, child_public_key: &[u8; 32]) -> ByteArray<64> {
        let mut msg = Vec::with_capacity(64);
        msg.extend_from_slice(&self.node_id);
        msg.extend_from_slice(child_public_key);
        ByteArray::from(self.sign(&msg))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, IdentityError> {
        bincode::serialize(self)
            .map_err(|e: bincode::Error| IdentityError::SerializationError(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IdentityError> {
        bincode::deserialize(data)
            .map_err(|e: bincode::Error| IdentityError::SerializationError(e.to_string()))
    }
}

impl fmt::Display for NodeIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Node {} (gen {}, {:?})",
            &hex::encode(self.node_id)[..8].to_uppercase(),
            self.generation,
            self.network_mode,
        )
    }
}

// --------------- Invitation Package ---------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationPackage {
    pub parent_id: [u8; 32],
    pub parent_multiaddr: String,
    pub network_mode: NetworkMode,
    pub welcome_signature: ByteArray<64>,
    pub expires_at: u64,
    pub target_generation: u32,
}

impl InvitationPackage {
    pub fn new(parent: &NodeIdentity, parent_multiaddr: String, expires_in_hours: u64) -> Self {
        let welcome_signature = parent.sign_welcome(&[0u8; 32]);
        Self {
            parent_id: parent.node_id,
            parent_multiaddr,
            network_mode: parent.network_mode,
            welcome_signature,
            expires_at: now_secs() + (expires_in_hours * 3600),
            target_generation: parent.generation + 1,
        }
    }

    pub fn is_valid(&self) -> bool {
        now_secs() < self.expires_at
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, IdentityError> {
        bincode::serialize(self)
            .map_err(|e: bincode::Error| IdentityError::SerializationError(e.to_string()))
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, IdentityError> {
        bincode::deserialize(data)
            .map_err(|e: bincode::Error| IdentityError::SerializationError(e.to_string()))
    }

    pub fn to_qr_data(&self) -> Result<String, IdentityError> {
        use base64::Engine;
        let bytes = self.to_bytes()?;
        Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
    }

    pub fn from_qr_data(qr_data: &str) -> Result<Self, IdentityError> {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(qr_data)
            .map_err(|e| IdentityError::SerializationError(e.to_string()))?;
        Self::from_bytes(&bytes)
    }
}

// --------------- Tests ---------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_identity() {
        let g = NodeIdentity::genesis(NetworkMode::Devnet);
        assert_eq!(g.generation, 0);
        assert!(g.parent_id.is_none());
        assert_eq!(g.network_mode, NetworkMode::Devnet);
        assert_ne!(g.node_id, [0u8; 32]);
    }

    #[test]
    fn test_child_identity() {
        let parent = NodeIdentity::genesis(NetworkMode::Testnet);
        let child = NodeIdentity::child_of(&parent).unwrap();
        assert_eq!(child.generation, 1);
        assert_eq!(child.parent_id, Some(parent.node_id));
        assert_ne!(child.node_id, parent.node_id);
    }

    #[test]
    fn test_lineage_depth_limit() {
        let mut node = NodeIdentity::genesis(NetworkMode::Devnet);
        for _ in 0..100 {
            node = NodeIdentity::child_of(&node).unwrap();
        }
        assert!(NodeIdentity::child_of(&node).is_err());
    }

    #[test]
    fn test_identity_serialization_roundtrip() {
        let id = NodeIdentity::genesis(NetworkMode::Devnet);
        let bytes = id.to_bytes().unwrap();
        let id2 = NodeIdentity::from_bytes(&bytes).unwrap();
        assert_eq!(id.node_id, id2.node_id);
        assert_eq!(id.generation, id2.generation);
    }

    #[test]
    fn test_invitation_package() {
        let parent = NodeIdentity::genesis(NetworkMode::Mainnet);
        let inv = InvitationPackage::new(&parent, "/ip4/127.0.0.1/udp/7802".into(), 24);
        assert_eq!(inv.parent_id, parent.node_id);
        assert_eq!(inv.target_generation, 1);
        assert!(inv.is_valid());
    }

    #[test]
    fn test_invitation_expiration() {
        let parent = NodeIdentity::genesis(NetworkMode::Devnet);
        let mut inv = InvitationPackage::new(&parent, "/ip4/127.0.0.1/udp/7800".into(), 1);
        assert!(inv.is_valid());
        inv.expires_at = 0;
        assert!(!inv.is_valid());
    }

    #[test]
    fn test_qr_roundtrip() {
        let parent = NodeIdentity::genesis(NetworkMode::Testnet);
        let inv = InvitationPackage::new(&parent, "/ip4/127.0.0.1/udp/7801".into(), 24);
        let qr = inv.to_qr_data().unwrap();
        let inv2 = InvitationPackage::from_qr_data(&qr).unwrap();
        assert_eq!(inv.parent_id, inv2.parent_id);
        assert_eq!(inv.network_mode, inv2.network_mode);
    }

    #[test]
    fn test_sign_and_verify() {
        let id = NodeIdentity::genesis(NetworkMode::Devnet);
        let data = b"hello piva";
        let sig = id.sign(data);
        assert!(id.verify_signature(data, &sig).is_ok());
        assert!(id.verify_signature(b"tampered", &sig).is_err());
    }
}
