//! # PIVA Lineage Storage
//!
//! Self-contained redb-backed storage for node lineage and reputation.

use crate::identity::{NodeIdentity, IdentityError};
use piva_core::network::NetworkMode;
use redb::{Database, TableDefinition, ReadableTable};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use tracing::info;

// --------------- Error ---------------

#[derive(Error, Debug)]
pub enum LineageError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<redb::DatabaseError> for LineageError {
    fn from(e: redb::DatabaseError) -> Self { Self::DatabaseError(e.to_string()) }
}
impl From<redb::TableError> for LineageError {
    fn from(e: redb::TableError) -> Self { Self::DatabaseError(e.to_string()) }
}
impl From<redb::CommitError> for LineageError {
    fn from(e: redb::CommitError) -> Self { Self::DatabaseError(e.to_string()) }
}
impl From<redb::StorageError> for LineageError {
    fn from(e: redb::StorageError) -> Self { Self::DatabaseError(e.to_string()) }
}
impl From<redb::TransactionError> for LineageError {
    fn from(e: redb::TransactionError) -> Self { Self::DatabaseError(e.to_string()) }
}

// --------------- Tables (key = hex string, value = bincode bytes) ---------------

pub const LINEAGE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("lineage");
pub const REPUTATION_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("reputation");

// --------------- Data structs ---------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEntry {
    pub node_id: [u8; 32],
    pub parent_id: Option<[u8; 32]>,
    pub generation: u32,
    pub network_mode: String,
    pub created_at: u64,
    pub last_activity: u64,
    pub child_count: u32,
    pub descendant_count: u32,
    pub status: NodeStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationEntry {
    pub node_id: [u8; 32],
    pub reputation_score: i32,
    pub successful_shares: u64,
    pub failed_shares: u64,
    pub successful_verifications: u64,
    pub failed_verifications: u64,
    pub children_invited: u32,
    pub children_active: u32,
    pub last_updated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeStatus {
    Active,
    Offline,
    Suspended,
    Removed,
}

// --------------- LineageStats ---------------

#[derive(Debug, Default, Clone)]
pub struct LineageStats {
    pub total_nodes: u32,
    pub total_generations: u32,
}

// --------------- Storage ---------------

pub struct LineageStorage {
    pub(crate) db: Database,
}

fn hex_key(id: &[u8; 32]) -> String {
    hex::encode(id)
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl LineageStorage {
    /// Open (or create) the lineage database at the given path.
    pub async fn new(data_dir: PathBuf, _network_mode: NetworkMode) -> Result<Self, LineageError> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| LineageError::DatabaseError(e.to_string()))?;
        let db_path = data_dir.join("lineage.redb");
        let db = Database::create(db_path)
            .map_err(|e| LineageError::DatabaseError(e.to_string()))?;
        Ok(Self { db })
    }

    /// Get raw database reference for testing purposes
    pub fn raw_db(&self) -> &redb::Database {
        &self.db
    }

    // ---- Lineage CRUD ----

    pub async fn store_node(&self, identity: &NodeIdentity) -> Result<(), LineageError> {
        let entry = LineageEntry {
            node_id: identity.node_id,
            parent_id: identity.parent_id,
            generation: identity.generation,
            network_mode: format!("{:?}", identity.network_mode),
            created_at: identity.created_at,
            last_activity: identity.created_at,
            child_count: 0,
            descendant_count: 0,
            status: NodeStatus::Active,
        };
        let key = hex_key(&identity.node_id);
        let bytes = bincode::serialize(&entry)
            .map_err(|e| LineageError::SerializationError(e.to_string()))?;

        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LINEAGE_TABLE)?;
            table.insert(key.as_str(), bytes.as_slice())?;
        }
        txn.commit()?;

        // Update parent child_count
        if let Some(pid) = identity.parent_id {
            self.increment_child_count(&pid).await?;
        }

        // Init reputation
        let rep = ReputationEntry {
            node_id: identity.node_id,
            reputation_score: 500,
            successful_shares: 0,
            failed_shares: 0,
            successful_verifications: 0,
            failed_verifications: 0,
            children_invited: 0,
            children_active: 0,
            last_updated: now_secs(),
        };
        self.put_reputation(&rep)?;

        info!("Stored node {} gen {}", &key[..8].to_uppercase(), identity.generation);
        Ok(())
    }

    pub async fn get_node(&self, node_id: &[u8; 32]) -> Result<Option<LineageEntry>, LineageError> {
        let key = hex_key(node_id);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(LINEAGE_TABLE)?;
        match table.get(key.as_str())? {
            Some(guard) => {
                let entry: LineageEntry = bincode::deserialize(guard.value())
                    .map_err(|e| LineageError::SerializationError(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub async fn get_children(&self, parent_id: &[u8; 32]) -> Result<Vec<LineageEntry>, LineageError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(LINEAGE_TABLE)?;
        let mut children = Vec::new();
        let iter = table.iter()?;
        for item in iter {
            let (key_guard, value_guard) = item?;
            let _key = key_guard.value(); // Not used, but keeps lock
            let entry: LineageEntry = bincode::deserialize(value_guard.value())
                .map_err(|e| LineageError::SerializationError(e.to_string()))?;
            if entry.parent_id == Some(*parent_id) {
                children.push(entry);
            }
        }
        Ok(children)
    }

    pub async fn get_lineage_path(&self, node_id: &[u8; 32]) -> Result<Vec<LineageEntry>, LineageError> {
        let mut path = Vec::new();
        let mut current = *node_id;
        while let Some(entry) = self.get_node(&current).await? {
            path.insert(0, entry.clone());
            match entry.parent_id {
                Some(pid) => current = pid,
                None => break,
            }
        }
        Ok(path)
    }

    pub async fn update_activity(&self, node_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut entry) = self.get_node(node_id).await? {
            entry.last_activity = now_secs();
            self.put_lineage(&entry)?;
        }
        Ok(())
    }

    pub async fn increment_child_count(&self, parent_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut entry) = self.get_node(parent_id).await? {
            entry.child_count += 1;
            self.put_lineage(&entry)?;
        }
        Ok(())
    }

    // ---- Reputation ----

    pub async fn get_reputation(&self, node_id: &[u8; 32]) -> Result<Option<ReputationEntry>, LineageError> {
        let key = hex_key(node_id);
        let txn = self.db.begin_read()?;
        let table = txn.open_table(REPUTATION_TABLE)?;
        match table.get(key.as_str())? {
            Some(guard) => {
                let entry: ReputationEntry = bincode::deserialize(guard.value())
                    .map_err(|e| LineageError::SerializationError(e.to_string()))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    pub async fn record_successful_share(&self, node_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut rep) = self.get_reputation(node_id).await? {
            rep.successful_shares += 1;
            rep.reputation_score += 5;
            rep.last_updated = now_secs();
            self.put_reputation(&rep)?;
        }
        Ok(())
    }

    pub async fn record_failed_share(&self, node_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut rep) = self.get_reputation(node_id).await? {
            rep.failed_shares += 1;
            rep.reputation_score -= 2;
            rep.last_updated = now_secs();
            self.put_reputation(&rep)?;
        }
        Ok(())
    }

    pub async fn record_successful_verification(&self, node_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut rep) = self.get_reputation(node_id).await? {
            rep.successful_verifications += 1;
            rep.reputation_score += 3;
            rep.last_updated = now_secs();
            self.put_reputation(&rep)?;
        }
        Ok(())
    }

    pub async fn record_failed_verification(&self, node_id: &[u8; 32]) -> Result<(), LineageError> {
        if let Some(mut rep) = self.get_reputation(node_id).await? {
            rep.failed_verifications += 1;
            rep.reputation_score -= 1;
            rep.last_updated = now_secs();
            self.put_reputation(&rep)?;
        }
        Ok(())
    }

    pub async fn store_invitation(&self, _invitation: &crate::identity::InvitationPackage) -> Result<(), LineageError> {
        // Placeholder — invitations are ephemeral for now
        Ok(())
    }

    // ---- Internal helpers ----

    fn put_lineage(&self, entry: &LineageEntry) -> Result<(), LineageError> {
        let key = hex_key(&entry.node_id);
        let bytes = bincode::serialize(entry)
            .map_err(|e| LineageError::SerializationError(e.to_string()))?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(LINEAGE_TABLE)?;
            table.insert(key.as_str(), bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    fn put_reputation(&self, entry: &ReputationEntry) -> Result<(), LineageError> {
        let key = hex_key(&entry.node_id);
        let bytes = bincode::serialize(entry)
            .map_err(|e| LineageError::SerializationError(e.to_string()))?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(REPUTATION_TABLE)?;
            table.insert(key.as_str(), bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_lineage_store_and_retrieve() {
        let tmp = TempDir::new().unwrap();
        let store = LineageStorage::new(tmp.path().to_path_buf(), NetworkMode::Devnet).await.unwrap();

        let genesis = NodeIdentity::genesis(NetworkMode::Devnet);
        store.store_node(&genesis).await.unwrap();

        let entry = store.get_node(&genesis.node_id).await.unwrap().unwrap();
        assert_eq!(entry.generation, 0);
        assert!(entry.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_parent_child_lineage() {
        let tmp = TempDir::new().unwrap();
        let store = LineageStorage::new(tmp.path().to_path_buf(), NetworkMode::Devnet).await.unwrap();

        let parent = NodeIdentity::genesis(NetworkMode::Devnet);
        store.store_node(&parent).await.unwrap();

        let child = NodeIdentity::child_of(&parent).unwrap();
        store.store_node(&child).await.unwrap();

        let children = store.get_children(&parent.node_id).await.unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].node_id, child.node_id);

        let path = store.get_lineage_path(&child.node_id).await.unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0].node_id, parent.node_id);
        assert_eq!(path[1].node_id, child.node_id);
    }

    #[tokio::test]
    async fn test_reputation_tracking() {
        let tmp = TempDir::new().unwrap();
        let store = LineageStorage::new(tmp.path().to_path_buf(), NetworkMode::Testnet).await.unwrap();

        let node = NodeIdentity::genesis(NetworkMode::Testnet);
        store.store_node(&node).await.unwrap();

        store.record_successful_share(&node.node_id).await.unwrap();
        store.record_successful_verification(&node.node_id).await.unwrap();
        store.record_failed_share(&node.node_id).await.unwrap();

        let rep = store.get_reputation(&node.node_id).await.unwrap().unwrap();
        assert_eq!(rep.successful_shares, 1);
        assert_eq!(rep.successful_verifications, 1);
        assert_eq!(rep.failed_shares, 1);
        // 500 + 5 + 3 - 2 = 506
        assert_eq!(rep.reputation_score, 506);
    }
}
