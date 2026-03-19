//! # Network Configuration
//! 
//! Configuration for different network modes with resource limits.

use piva_core::network::NetworkMode;

/// Network configuration for PIVA nodes
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub mode: NetworkMode,
    pub port: u16,
    pub max_connections: usize,
    pub buffer_size: usize,
    pub magic_byte: u8,
}

impl NetworkConfig {
    /// Create configuration for a specific network mode
    pub fn new(mode: NetworkMode) -> Self {
        Self {
            port: mode.port(),
            max_connections: mode.max_connections(),
            buffer_size: mode.buffer_size(),
            magic_byte: mode.magic_byte(),
            mode,
        }
    }
    
    /// Get default configuration for development
    pub fn devnet() -> Self {
        Self::new(NetworkMode::Devnet)
    }
    
    /// Get default configuration for testing
    pub fn testnet() -> Self {
        Self::new(NetworkMode::Testnet)
    }
    
    /// Get default configuration for production
    pub fn mainnet() -> Self {
        Self::new(NetworkMode::Mainnet)
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::devnet()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use piva_core::network::NetworkMode;
    
    #[test]
    fn test_config_creation() {
        let config = NetworkConfig::new(NetworkMode::Testnet);
        assert_eq!(config.mode, NetworkMode::Testnet);
        assert_eq!(config.port, 7801);
        assert_eq!(config.max_connections, 25);
        assert_eq!(config.buffer_size, 8192);
        assert_eq!(config.magic_byte, 0x02);
    }
    
    #[test]
    fn test_config_defaults() {
        assert_eq!(NetworkConfig::default().mode, NetworkMode::Devnet);
        assert_eq!(NetworkConfig::devnet().mode, NetworkMode::Devnet);
        assert_eq!(NetworkConfig::testnet().mode, NetworkMode::Testnet);
        assert_eq!(NetworkConfig::mainnet().mode, NetworkMode::Mainnet);
    }
}
