//! # PIVA P2P Network Layer
//! 
//! This module provides peer-to-peer networking using Iroh with
//! content-addressed storage and network mode isolation.

pub mod node;
pub mod config;
pub mod identity;
pub mod lineage;

#[cfg(test)]
pub mod tests;

pub use node::PivaNode;
pub use config::NetworkConfig;
pub use identity::{NodeIdentity, InvitationPackage, IdentityError};
pub use lineage::{LineageStorage, LineageEntry, ReputationEntry, NodeStatus, LineageError};
