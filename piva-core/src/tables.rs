//! # Tables Module
//!
//! Table definitions for piva-core storage operations.

use redb::TableDefinition;

/// Multimedia table - stores optimized audio/video chunks
pub const MULTIMEDIA_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("multimedia");
