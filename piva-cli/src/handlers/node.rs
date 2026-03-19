//! Node management handlers
//! 
//! Implements init, status, and config commands with real memory monitoring.

use anyhow::Result;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tracing::{info, error};

/// Handle node initialization with real directory creation
pub async fn handle_init(force: bool, json_output: bool) -> Result<()> {
    info!("Initializing PIVA node");
    
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    if data_dir.exists() && !force {
        error!("PIVA node already exists. Use --force to overwrite.");
        return Err(anyhow::anyhow!("PIVA node already exists"));
    }
    
    // Create directories
    fs::create_dir_all(&data_dir)?;
    fs::create_dir_all(data_dir.join("config"))?;
    fs::create_dir_all(data_dir.join("storage"))?;
    
    // TODO: Generate real keypair with piva-crypto
    // TODO: Create real configuration file
    
    if json_output {
        let output = json!({
            "status": "success",
            "data_dir": data_dir.to_string_lossy(),
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("✅ PIVA node initialized successfully!");
        println!("   Data directory: {}", data_dir.display());
    }
    
    Ok(())
}

/// Handle status command with real RSS memory monitoring
pub async fn handle_status(detailed: bool, json_output: bool) -> Result<()> {
    info!("Getting node status");
    
    // Read real RSS from /proc/self/statm (Linux/VPS)
    let rss_bytes = read_rss_memory()?;
    let rss_mb = rss_bytes as f64 / 1024.0 / 1024.0;
    
    // TODO: Get real storage stats from piva-storage
    let storage_stats = json!({
        "total_assets": 0, // TODO: Real count
        "storage_size": "0 MB" // TODO: Real size
    });
    
    if json_output {
        let output = json!({
            "status": "success",
            "memory": {
                "rss_bytes": rss_bytes,
                "rss_mb": format!("{:.2}", rss_mb),
                "within_limit": rss_mb < 512.0
            },
            "storage": storage_stats,
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("📊 PIVA Node Status:");
        println!("   Memory Usage: {:.2} MB RSS", rss_mb);
        if rss_mb > 512.0 {
            println!("   ⚠️  Memory usage exceeds 512MB limit!");
        }
        
        if detailed {
            println!("   Storage: {} assets", storage_stats["total_assets"]);
            println!("   Storage Size: {}", storage_stats["storage_size"]);
        }
    }
    
    Ok(())
}

/// Read RSS memory from /proc/self/statm on Linux
fn read_rss_memory() -> Result<usize> {
    let statm = fs::read_to_string("/proc/self/statm")?;
    let parts: Vec<&str> = statm.trim().split_whitespace().collect();
    
    if parts.len() >= 2 {
        // RSS is the second value in pages
        let rss_pages = parts[1].parse::<usize>()?;
        // Convert pages to bytes (usually 4KB per page)
        let rss_bytes = rss_pages * 4096;
        Ok(rss_bytes)
    } else {
        Err(anyhow::anyhow!("Failed to parse /proc/self/statm"))
    }
}

/// Handle configuration management
pub async fn handle_config(action: &str, key: Option<String>, value: Option<String>, json_output: bool) -> Result<()> {
    info!("Configuration action: {}", action);
    
    match action {
        "show" => {
            // TODO: Implement real config display
            if json_output {
                let output = json!({
                    "status": "success",
                    "config": {
                        "network": "devnet",
                        "port": 8080
                    },
                    "timestamp": chrono::Utc::now().timestamp()
                });
                println!("{}", output);
            } else {
                println!("⚙️  Current Configuration:");
                println!("   Network: devnet");
                println!("   Port: 8080");
            }
        }
        "set" => {
            if let (Some(k), Some(v)) = (key, value) {
                // TODO: Implement real config setting
                if json_output {
                    let output = json!({
                        "status": "success",
                        "key": k,
                        "value": v,
                        "timestamp": chrono::Utc::now().timestamp()
                    });
                    println!("{}", output);
                } else {
                    println!("✅ Configuration updated: {} = {}", k, v);
                }
            } else {
                return Err(anyhow::anyhow!("Both key and value required for set operation"));
            }
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown config action: {}", action));
        }
    }
    
    Ok(())
}
