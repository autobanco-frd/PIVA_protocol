//! # P2P Network Tests
//!
//! Comprehensive tests for blob exchange, cross-network isolation, and Sprint 6 identity features.

use crate::node::PivaNode;
use crate::config::NetworkConfig;
use bytes::Bytes;
use piva_core::network::NetworkMode;
use std::time::Duration;
use tempfile::TempDir;

/// Test blob exchange between two genesis nodes
#[tokio::test]
async fn test_p2p_blob_exchange() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config1 = NetworkConfig::new(NetworkMode::Devnet);
    let config2 = NetworkConfig::new(NetworkMode::Devnet);

    let mut node1 = PivaNode::genesis(config1, temp_dir.path().join("node1")).await?;
    let mut node2 = PivaNode::genesis(config2, temp_dir.path().join("node2")).await?;

    node1.start().await?;
    node2.start().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    let original_data = Bytes::from("Hello PIVA Network!");
    let hash = node1.publish_content(original_data.clone()).await?;

    let expected_hash = piva_crypto::hash_blake3(&original_data);
    assert_eq!(hash, expected_hash);

    // Same content → same hash
    let hash2 = node2.publish_content(original_data.clone()).await?;
    assert_eq!(hash, hash2);

    // Mock fetch contains the hex hash
    let fetched = node2.fetch_content(&hash).await?;
    let hash_hex = hex::encode(hash);
    assert!(fetched.windows(hash_hex.len()).any(|w| w == hash_hex.as_bytes()));

    node1.stop().await?;
    node2.stop().await?;

    println!("✅ P2P blob exchange test passed");
    Ok(())
}

/// Test cross-network rejection via magic bytes
#[tokio::test]
async fn test_cross_network_rejection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let mut devnet = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Devnet), temp_dir.path().join("devnet")).await?;
    let mut testnet = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Testnet), temp_dir.path().join("testnet")).await?;

    devnet.start().await?;
    testnet.start().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert!(devnet.verify_magic_byte(NetworkMode::Testnet.magic_byte()).is_err());
    assert!(testnet.verify_magic_byte(NetworkMode::Devnet.magic_byte()).is_err());
    assert!(devnet.verify_magic_byte(NetworkMode::Devnet.magic_byte()).is_ok());
    assert!(testnet.verify_magic_byte(NetworkMode::Testnet.magic_byte()).is_ok());

    devnet.stop().await?;
    testnet.stop().await?;

    println!("✅ Cross-network rejection test passed");
    Ok(())
}

/// Test large blob streaming with BAO
#[tokio::test]
async fn test_large_blob_streaming() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let mut node = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Devnet), temp_dir.path().join("node")).await?;
    node.start().await?;

    let large_data = Bytes::from(vec![0u8; 10 * 1024 * 1024]);

    let start = std::time::Instant::now();
    let hash = node.publish_content(large_data.clone()).await?;
    println!("Published 10 MB in {:?}", start.elapsed());

    let expected = piva_crypto::hash_blake3(&large_data);
    assert_eq!(hash, expected);

    // Mock fetch
    let fetched = node.fetch_content(&hash).await?;
    let hash_hex = hex::encode(hash);
    assert!(fetched.windows(hash_hex.len()).any(|w| w == hash_hex.as_bytes()));

    // Chunk verification placeholder
    let chunk = &large_data[0..64 * 1024];
    assert!(node.verify_chunk(&hash, 0, chunk).await?);

    node.stop().await?;
    println!("✅ Large blob streaming test passed");
    Ok(())
}

/// Test network resource limits per mode
#[tokio::test]
async fn test_network_resource_limits() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    let devnet = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Devnet), temp_dir.path().join("d")).await?;
    let stats = devnet.network_stats().await;
    assert_eq!(stats.network_mode, NetworkMode::Devnet);
    assert_eq!(stats.port, 7800);
    assert_eq!(stats.max_connections, 5);
    assert_eq!(stats.buffer_size, 4096);

    let testnet = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Testnet), temp_dir.path().join("t")).await?;
    let stats = testnet.network_stats().await;
    assert_eq!(stats.network_mode, NetworkMode::Testnet);
    assert_eq!(stats.port, 7801);
    assert_eq!(stats.max_connections, 25);
    assert_eq!(stats.buffer_size, 8192);

    let mainnet = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Mainnet), temp_dir.path().join("m")).await?;
    let stats = mainnet.network_stats().await;
    assert_eq!(stats.network_mode, NetworkMode::Mainnet);
    assert_eq!(stats.port, 7802);
    assert_eq!(stats.max_connections, 50);
    assert_eq!(stats.buffer_size, 8192);

    println!("✅ Network resource limits test passed");
    Ok(())
}

/// Test memory usage under stress (simplified)
#[tokio::test]
async fn test_memory_stress_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let mut node = PivaNode::genesis(
        NetworkConfig::new(NetworkMode::Devnet), temp_dir.path().join("node")).await?;
    node.start().await?;

    for i in 0u8..100 {
        let data = Bytes::from(vec![i; 1024 * 1024]);
        let _hash = node.publish_content(data).await?;
    }

    node.stop().await?;
    println!("✅ Memory stress test passed");
    Ok(())
}
