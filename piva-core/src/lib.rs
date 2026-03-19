//! # PIVA Core Data Structures
//! 
//! This module defines the fundamental data structures used throughout
//! the PIVA protocol for representing assets, metadata, and network modes.

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

pub mod asset;
pub mod network;
pub mod rwa;
pub mod scoring;
pub mod swap;
pub mod made;
pub mod cache;
pub mod multimedia;
pub mod tables;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/piva.core.rs"));
}

pub use asset::{AssetId, AssetType, AssetMetadata, AssetEntry};
pub use network::NetworkMode;
pub use scoring::{PeerScore, Achievement, TrustLevel, AchievementAggregator, ScoringError};
pub use made::{MadeAgent, MadeConfig, MadeDecision, ResourceMetrics, MadeError};
pub use rwa::{RwaAsset, RwaAssetType, AudioFormat, VideoFormat, VerifiedChunk, RevocationCertificate, RwaError};
pub use cache::ChunkCache;
pub use multimedia::{MultimediaStorage, MultimediaConfig, StorageStats, MultimediaError};
