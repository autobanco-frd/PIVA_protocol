//! # Network Mode Configuration
//! 
//! Defines the different network modes (Devnet, Testnet, Mainnet) with
//! their respective prefixes and configurations.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Network mode for PIVA protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetworkMode {
    /// Development network - ephemeral, in-memory storage
    Devnet,
    /// Test network - public but no real value
    Testnet,
    /// Main network - production with real value
    Mainnet,
}

impl NetworkMode {
    /// Get the port for this network mode
    pub fn port(&self) -> u16 {
        match self {
            NetworkMode::Devnet => 7800,
            NetworkMode::Testnet => 7801,
            NetworkMode::Mainnet => 7802,
        }
    }
    
    /// Get maximum connections for this network mode
    pub fn max_connections(&self) -> usize {
        match self {
            NetworkMode::Devnet => 5,
            NetworkMode::Testnet => 25,
            NetworkMode::Mainnet => 50,
        }
    }
    
    /// Get buffer size in bytes for this network mode
    pub fn buffer_size(&self) -> usize {
        match self {
            NetworkMode::Devnet => 4096,  // 4 KB
            NetworkMode::Testnet => 8192, // 8 KB
            NetworkMode::Mainnet => 8192,  // 8 KB
        }
    }
    
    /// Get magic byte for network isolation
    pub fn magic_byte(&self) -> u8 {
        match self {
            NetworkMode::Devnet => 0x01,
            NetworkMode::Testnet => 0x02,
            NetworkMode::Mainnet => 0x03,
        }
    }
    /// Get the prefix for asset IDs in this network mode
    pub fn prefix(&self) -> &'static str {
        match self {
            NetworkMode::Devnet => "piva_dev_",
            NetworkMode::Testnet => "piva_test_",
            NetworkMode::Mainnet => "piva_live_",
        }
    }
    
    /// Check if this is a production network
    pub fn is_production(&self) -> bool {
        matches!(self, NetworkMode::Mainnet)
    }
    
    /// Check if this network uses disk persistence
    pub fn uses_disk_persistence(&self) -> bool {
        !matches!(self, NetworkMode::Devnet)
    }
}

impl fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkMode::Devnet => write!(f, "devnet"),
            NetworkMode::Testnet => write!(f, "testnet"),
            NetworkMode::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl std::str::FromStr for NetworkMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "devnet" => Ok(NetworkMode::Devnet),
            "testnet" => Ok(NetworkMode::Testnet),
            "mainnet" => Ok(NetworkMode::Mainnet),
            _ => Err(format!("Invalid network mode: {}. Expected: devnet, testnet, mainnet", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_prefixes() {
        assert_eq!(NetworkMode::Devnet.prefix(), "piva_dev_");
        assert_eq!(NetworkMode::Testnet.prefix(), "piva_test_");
        assert_eq!(NetworkMode::Mainnet.prefix(), "piva_live_");
    }
    
    #[test]
    fn test_network_ports() {
        assert_eq!(NetworkMode::Devnet.port(), 7800);
        assert_eq!(NetworkMode::Testnet.port(), 7801);
        assert_eq!(NetworkMode::Mainnet.port(), 7802);
    }
    
    #[test]
    fn test_magic_bytes() {
        assert_eq!(NetworkMode::Devnet.magic_byte(), 0x01);
        assert_eq!(NetworkMode::Testnet.magic_byte(), 0x02);
        assert_eq!(NetworkMode::Mainnet.magic_byte(), 0x03);
    }
    
    #[test]
    fn test_connection_limits() {
        assert_eq!(NetworkMode::Devnet.max_connections(), 5);
        assert_eq!(NetworkMode::Testnet.max_connections(), 25);
        assert_eq!(NetworkMode::Mainnet.max_connections(), 50);
    }
    
    #[test]
    fn test_buffer_sizes() {
        assert_eq!(NetworkMode::Devnet.buffer_size(), 4096);
        assert_eq!(NetworkMode::Testnet.buffer_size(), 8192);
        assert_eq!(NetworkMode::Mainnet.buffer_size(), 8192);
    }
    
    #[test]
    fn test_production_check() {
        assert!(!NetworkMode::Devnet.is_production());
        assert!(!NetworkMode::Testnet.is_production());
        assert!(NetworkMode::Mainnet.is_production());
    }
    
    #[test]
    fn test_persistence_check() {
        assert!(!NetworkMode::Devnet.uses_disk_persistence());
        assert!(NetworkMode::Testnet.uses_disk_persistence());
        assert!(NetworkMode::Mainnet.uses_disk_persistence());
    }
    
    #[test]
    fn test_from_str() {
        assert_eq!("devnet".parse::<NetworkMode>().unwrap(), NetworkMode::Devnet);
        assert_eq!("DEVNET".parse::<NetworkMode>().unwrap(), NetworkMode::Devnet);
        assert_eq!("testnet".parse::<NetworkMode>().unwrap(), NetworkMode::Testnet);
        assert_eq!("mainnet".parse::<NetworkMode>().unwrap(), NetworkMode::Mainnet);
        
        assert!("invalid".parse::<NetworkMode>().is_err());
    }
    
    #[test]
    fn test_display() {
        assert_eq!(NetworkMode::Devnet.to_string(), "devnet");
        assert_eq!(NetworkMode::Testnet.to_string(), "testnet");
        assert_eq!(NetworkMode::Mainnet.to_string(), "mainnet");
    }
}
