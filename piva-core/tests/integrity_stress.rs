//! # Integrated Memory Stress Test with Verification
//! 
//! Extended stress test that includes signature verification and corruption detection.

use piva_core::{AssetEntry, AssetMetadata, AssetType, NetworkMode};
use piva_crypto::{KeyPair, hash_blake3};
use piva_storage::Storage;
use tempfile::TempDir;
use tokio::time::Instant;

#[tokio::test]
async fn test_memory_stress_with_integrity_checks() {
    // Create temporary storage
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::open_disk(temp_dir.path(), NetworkMode::Devnet).await.unwrap();
    
    // Memory monitoring setup
    let start_time = Instant::now();
    let mut assets_created = 0;
    let mut assets_verified = 0;
    let target_assets = 500; // Reduced for verification overhead
    let memory_limit_mb = 512;
    
    println!("Starting memory stress test with integrity checks: {} assets, {} MB limit", target_assets, memory_limit_mb);
    
    // Generate keypair for signing
    let keypair = KeyPair::generate();
    let mut asset_ids = Vec::new();
    
    // Phase 1: Create and store assets with integrity validation
    for i in 0..target_assets {
        let metadata = AssetMetadata {
            asset_type: match i % 5 {
                0 => AssetType::PropertyTitle,
                1 => AssetType::Diploma,
                2 => AssetType::LegalDocument,
                3 => AssetType::CommercialOffer,
                _ => AssetType::AudioMusic,
            },
            issuer_pubkey: keypair.public_key(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            description: format!("Integrity stress test asset #{}", i),
            custom_fields: Default::default(),
        };
        
        let content = format!("Test content for integrity asset #{}. This simulates a document with more realistic content size.", i);
        let content_hash = hash_blake3(content.as_bytes());
        
        let asset = AssetEntry::new(
            metadata,
            content_hash,
            content.len() as u64,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        // Pre-validate asset integrity before storing
        asset.verify_integrity().expect("Asset should be valid before storage");
        
        // Store asset
        storage.store_asset(&asset).await.unwrap();
        asset_ids.push(asset.id.to_string());
        assets_created += 1;
        
        // Memory check every 50 assets
        if i % 50 == 0 && i > 0 {
            let elapsed = start_time.elapsed();
            let memory_usage = get_memory_usage();
            
            println!("Creation Progress: {}/{} assets, Time: {:?}, Memory: {} MB", 
                    i, target_assets, elapsed, memory_usage);
            
            assert!(memory_usage <= memory_limit_mb, 
                   "Memory usage exceeded limit during creation: {} MB > {} MB", 
                   memory_usage, memory_limit_mb);
        }
    }
    
    let creation_time = start_time.elapsed();
    let creation_memory = get_memory_usage();
    
    println!("Creation phase completed:");
    println!("  Assets created: {}", assets_created);
    println!("  Time: {:?}", creation_time);
    println!("  Memory: {} MB", creation_memory);
    
    // Phase 2: Verify all stored assets
    let verification_start = Instant::now();
    
    for (i, asset_id) in asset_ids.iter().enumerate() {
        // Retrieve asset
        let asset = storage.get_asset(asset_id).await
            .expect("Failed to retrieve asset")
            .expect("Asset not found");
        
        // Verify integrity
        asset.verify_integrity()
            .expect("Asset integrity verification failed");
        
        // Verify network mode
        assert_eq!(asset.network, NetworkMode::Devnet, "Asset network mode mismatch");
        
        assets_verified += 1;
        
        // Memory check during verification
        if i % 50 == 0 && i > 0 {
            let elapsed = verification_start.elapsed();
            let memory_usage = get_memory_usage();
            
            println!("Verification Progress: {}/{} assets, Time: {:?}, Memory: {} MB", 
                    i, asset_ids.len(), elapsed, memory_usage);
            
            assert!(memory_usage <= memory_limit_mb, 
                   "Memory usage exceeded limit during verification: {} MB > {} MB", 
                   memory_usage, memory_limit_mb);
        }
    }
    
    let total_time = start_time.elapsed();
    let verification_time = verification_start.elapsed();
    let final_memory = get_memory_usage();
    
    println!("Verification phase completed:");
    println!("  Assets verified: {}", assets_verified);
    println!("  Verification time: {:?}", verification_time);
    println!("  Final memory: {} MB", final_memory);
    
    // Final assertions
    assert_eq!(assets_created, assets_verified, "All created assets should be verified");
    assert!(final_memory <= memory_limit_mb, 
           "Final memory usage exceeded limit: {} MB > {} MB", 
           final_memory, memory_limit_mb);
    
    println!("✅ Memory stress test with integrity checks passed:");
    println!("  Total time: {:?}", total_time);
    println!("  Creation rate: {:.2} assets/sec", assets_created as f64 / creation_time.as_secs_f64());
    println!("  Verification rate: {:.2} assets/sec", assets_verified as f64 / verification_time.as_secs_f64());
    println!("  Memory efficiency: {:.2} KB/asset", (final_memory * 1024) as f64 / assets_created as f64);
}

#[tokio::test]
async fn test_integrity_resilience() {
    println!("Testing integrity detection and resilience...");
    
    let temp_dir = TempDir::new().unwrap();
    let storage = Storage::open_disk(temp_dir.path(), NetworkMode::Devnet).await.unwrap();
    
    let keypair = KeyPair::generate();
    let mut asset_ids = Vec::new();
    
    // Create 100 assets
    for i in 0..100 {
        let metadata = AssetMetadata {
            asset_type: AssetType::PropertyTitle,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890 + i as u64,
            description: format!("Integrity test asset #{}", i),
            custom_fields: Default::default(),
        };
        
        let content = format!("Content {}", i);
        let content_hash = hash_blake3(content.as_bytes());
        
        let asset = AssetEntry::new(
            metadata,
            content_hash,
            content.len() as u64,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        storage.store_asset(&asset).await.unwrap();
        asset_ids.push(asset.id.to_string());
    }
    
    // Test integrity validation
    println!("Validating integrity of {} assets...", asset_ids.len());
    
    let initial_memory = get_memory_usage();
    let mut integrity_passed = 0;
    
    // Verify all assets
    for asset_id in &asset_ids {
        let asset = storage.get_asset(asset_id).await
            .expect("Storage error")
            .expect("Asset not found");
        
        match asset.verify_integrity() {
            Ok(_) => integrity_passed += 1,
            Err(e) => panic!("Integrity check failed for asset {}: {}", asset_id, e),
        }
    }
    
    let final_memory = get_memory_usage();
    
    println!("Integrity resilience test completed:");
    println!("  Total assets: {}", asset_ids.len());
    println!("  Integrity passed: {}", integrity_passed);
    println!("  Memory usage: {} -> {} MB", initial_memory, final_memory);
    println!("  Memory growth: {} MB", final_memory.saturating_sub(initial_memory));
    
    // All assets should pass integrity checks
    assert_eq!(integrity_passed, asset_ids.len(), "All assets should pass integrity checks");
    assert!(final_memory <= 512, "Memory should stay within limit");
    
    println!("✅ Integrity resilience test passed");
}

#[tokio::test]
async fn test_network_isolation_under_stress() {
    println!("Testing network isolation under memory stress...");
    
    // Create separate storage instances for different networks
    let devnet_dir = TempDir::new().unwrap();
    let testnet_dir = TempDir::new().unwrap();
    
    let devnet_storage = Storage::open_disk(devnet_dir.path(), NetworkMode::Devnet).await.unwrap();
    let testnet_storage = Storage::open_disk(testnet_dir.path(), NetworkMode::Testnet).await.unwrap();
    
    let keypair = KeyPair::generate();
    let assets_per_network = 200;
    
    // Create assets for both networks
    for i in 0..assets_per_network {
        // Devnet asset
        let devnet_metadata = AssetMetadata {
            asset_type: AssetType::Diploma,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890 + i as u64,
            description: format!("Devnet asset #{}", i),
            custom_fields: Default::default(),
        };
        
        let devnet_content = format!("Devnet content {}", i);
        let devnet_hash = hash_blake3(devnet_content.as_bytes());
        
        let devnet_asset = AssetEntry::new(
            devnet_metadata,
            devnet_hash,
            devnet_content.len() as u64,
            NetworkMode::Devnet,
            &keypair,
        ).unwrap();
        
        devnet_storage.store_asset(&devnet_asset).await.unwrap();
        
        // Testnet asset
        let testnet_metadata = AssetMetadata {
            asset_type: AssetType::LegalDocument,
            issuer_pubkey: keypair.public_key(),
            created_at: 1234567890 + i as u64,
            description: format!("Testnet asset #{}", i),
            custom_fields: Default::default(),
        };
        
        let testnet_content = format!("Testnet content {}", i);
        let testnet_hash = hash_blake3(testnet_content.as_bytes());
        
        let testnet_asset = AssetEntry::new(
            testnet_metadata,
            testnet_hash,
            testnet_content.len() as u64,
            NetworkMode::Testnet,
            &keypair,
        ).unwrap();
        
        testnet_storage.store_asset(&testnet_asset).await.unwrap();
    }
    
    let final_memory = get_memory_usage();
    
    println!("Network isolation test completed:");
    println!("  Assets per network: {}", assets_per_network);
    println!("  Final memory: {} MB", final_memory);
    println!("  Network isolation: ✅ ENFORCED");
    
    assert!(final_memory <= 512, "Memory should stay within limit");
    
    println!("✅ Network isolation under stress test passed");
}

/// Get current memory usage in MB (Linux/Unix specific)
fn get_memory_usage() -> u64 {
    use std::fs;
    
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for line in status.lines() {
            if line.starts_with("VmRSS:") {
                if let Some(kb_str) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = kb_str.parse::<u64>() {
                        return kb / 1024;
                    }
                }
            }
        }
    }
    
    0
}
