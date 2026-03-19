//! # PIVA Storage Layer
//! 
//! This module provides persistent storage using `redb` with support for
//! different network modes and non-blocking operations suitable for
//! 512 MB RAM environments.

pub mod storage;
pub mod tables;
pub mod verified;
pub mod scoring;

pub use storage::Storage;
pub use tables::{ASSETS_TABLE, CONTENT_INDEX_TABLE, PEER_SCORES_TABLE, ACHIEVEMENTS_TABLE, TRANSFER_LOG_TABLE};
pub use verified::{VerifiedStorage, VerificationError};
pub use scoring::{ScoringStorage, NetworkStats};
