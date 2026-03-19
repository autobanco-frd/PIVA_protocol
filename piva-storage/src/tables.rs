//! # Database Table Definitions
//! 
//! Defines all redb tables used by PIVA for storage.

use redb::TableDefinition;

/// Assets table - stores serialized AssetEntry objects
pub const ASSETS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("assets");

/// Content index table - maps content hashes to asset IDs
pub const CONTENT_INDEX_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("content_idx");

/// Peer scores table - stores Web of Trust reputation data
pub const PEER_SCORES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("peer_scores");

/// Achievements table - stores achievement ledger data
pub const ACHIEVEMENTS_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("achievements");

/// Transfer log table - stores complete custody chain
pub const TRANSFER_LOG_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("transfer_log");

/// Multimedia table - stores optimized audio/video chunks
pub const MULTIMEDIA_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("multimedia");

#[cfg(test)]
mod tests {
    use super::*;
    use redb::TableHandle;
    
    #[test]
    fn test_table_definitions() {
        // Verify table names
        assert_eq!(ASSETS_TABLE.name(), "assets");
        assert_eq!(CONTENT_INDEX_TABLE.name(), "content_idx");
        assert_eq!(PEER_SCORES_TABLE.name(), "peer_scores");
        assert_eq!(TRANSFER_LOG_TABLE.name(), "transfer_log");
    }
}
