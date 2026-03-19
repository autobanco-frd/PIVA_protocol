//! Asset management handlers
//! 
//! Implements register, list, and verify commands for RWA assets with real integration.

use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;
use tracing::{info, error, debug};
use piva_core::{asset::{AssetEntry, AssetMetadata, AssetType}, network::NetworkMode};
use piva_crypto::{KeyPair, hash_sha3_256};
use piva_storage::Storage;
use chacha20poly1305::{
    aead::{Aead, AeadCore, OsRng}, // Añadimos AeadCore aquí
    ChaCha20Poly1305, Key, KeyInit,
};
use argon2::{Argon2, PasswordHasher};
use argon2::password_hash::SaltString; // Quitamos el segundo OsRng de aquí
use std::fs;
use base64::{engine::general_purpose, Engine as _};

/// Handle asset registration with real crypto and storage integration
pub async fn handle_register(path: String, asset_type: String, json_output: bool) -> Result<()> {
    info!("Registering asset: {} ({})", path, asset_type);
    
    // Validate and read the asset file
    let asset_path = PathBuf::from(&path);
    
    if !asset_path.exists() {
        error!("Asset file not found: {}", path);
        return Err(anyhow::anyhow!("Asset file not found: {}", path));
    }
    
    // Parse asset type
    let asset_type_enum = asset_type.parse::<AssetType>()
        .map_err(|e| anyhow::anyhow!("Invalid asset type: {}", e))?;
    
    // Generate or load keypair (for now, generate new one each time)
    // TODO: Implement persistent keypair management
    let keypair = KeyPair::generate();
    info!("Generated new keypair: {}", hex::encode(keypair.public_key()));
    
    // Read file content and calculate hash
    let file_content = tokio::fs::read(&asset_path).await?;
    let content_hash = hash_sha3_256(&file_content);
    let content_size = file_content.len() as u64;
    
    debug!("File size: {} bytes, hash: {}", content_size, hex::encode(content_hash));
    
    // Create asset metadata
    let metadata = AssetMetadata::new(
        asset_type_enum,
        keypair.public_key(),
        format!("Asset registered from file: {}", asset_path.display()),
    );
    
    // Determine network mode (for now, use devnet)
    // TODO: Get from CLI args or config
    let network = NetworkMode::Devnet;
    
    // Create asset entry with real signature
    let asset_entry = AssetEntry::new(
        metadata,
        content_hash,
        content_size,
        network,
        &keypair,
    ).map_err(|e| anyhow::anyhow!("Failed to create asset entry: {}", e))?;
    
    // Verify integrity before storing
    asset_entry.verify_integrity()
        .map_err(|e| anyhow::anyhow!("Asset integrity check failed: {}", e))?;
    
    // Store in database (use disk storage for persistence)
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let storage = Storage::open_disk(data_dir, network).await
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {}", e))?;
    
    storage.store_asset(&asset_entry).await
        .map_err(|e| anyhow::anyhow!("Failed to store asset: {}", e))?;
    
    info!("Asset successfully registered with ID: {}", asset_entry.id);
    
    if json_output {
        let output = json!({
            "status": "success",
            "asset_id": asset_entry.id.to_string(),
            "path": path,
            "asset_type": asset_type,
            "content_size": content_size,
            "content_hash": hex::encode(content_hash),
            "issuer_pubkey": hex::encode(keypair.public_key()),
            "network": "devnet",
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("✅ Asset registered successfully!");
        println!("   Asset ID: {}", asset_entry.id);
        println!("   Path: {}", path);
        println!("   Type: {}", asset_type);
        println!("   Size: {} bytes", content_size);
        println!("   Hash: {}", hex::encode(content_hash));
        println!("   Network: devnet");
    }
    
    Ok(())
}

/// Handle asset listing with real zero-copy iteration from redb
pub async fn handle_list(limit: usize, json_output: bool) -> Result<()> {
    info!("Listing up to {} assets", limit);
    
    // Open storage (use disk storage for persistence)
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let storage = Storage::open_disk(data_dir, NetworkMode::Devnet).await
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {}", e))?;
    
    // List assets with zero-copy iteration
    let assets = storage.list_assets(limit).await
        .map_err(|e| anyhow::anyhow!("Failed to list assets: {}", e))?;
    
    if json_output {
        let assets_json: Vec<serde_json::Value> = assets.iter().map(|asset| {
            json!({
                "id": asset.id.to_string(),
                "asset_type": asset.metadata.asset_type.as_str(),
                "description": asset.metadata.description,
                "content_size": asset.content_size,
                "content_hash": hex::encode(asset.content_hash),
                "issuer_pubkey": hex::encode(asset.metadata.issuer_pubkey),
                "created_at": asset.metadata.created_at,
                "network": asset.network.to_string()
            })
        }).collect();
        
        let output = json!({
            "status": "success",
            "assets": assets_json,
            "count": assets.len(),
            "limit": limit,
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("📋 Local Assets ({}):", assets.len());
        for (i, asset) in assets.iter().enumerate() {
            println!("   {}. {} ({})", 
                i + 1, 
                asset.id, 
                asset.metadata.asset_type
            );
            println!("      Size: {} bytes", asset.content_size);
            println!("      Created: {}", 
                chrono::DateTime::from_timestamp(asset.metadata.created_at as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S")
            );
        }
    }
    
    Ok(())
}

/// Handle asset verification with real Ed25519 signature check
pub async fn handle_verify(id: String, json_output: bool) -> Result<()> {
    info!("Verifying asset: {}", id);
    
    // Parse asset ID
    let asset_id = id.parse::<piva_core::asset::AssetId>()
        .map_err(|e| anyhow::anyhow!("Invalid asset ID: {}", e))?;
    
    // Open storage (use disk storage for persistence)
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let storage = Storage::open_disk(data_dir, asset_id.network().unwrap_or(NetworkMode::Devnet)).await
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {}", e))?;
    
    // Retrieve asset
    let asset = storage.get_asset(&asset_id.to_string()).await
        .map_err(|e| anyhow::anyhow!("Failed to retrieve asset: {}", e))?;
    
    let asset = match asset {
        Some(asset) => asset,
        None => {
            error!("Asset not found: {}", id);
            if json_output {
                let output = json!({
                    "status": "error",
                    "asset_id": id,
                    "error": "Asset not found",
                    "valid": false,
                    "timestamp": chrono::Utc::now().timestamp()
                });
                println!("{}", output);
            } else {
                println!("❌ Asset not found: {}", id);
            }
            std::process::exit(1);
        }
    };
    
    // Verify integrity with real Ed25519 signature check
    let is_valid = asset.verify_integrity().is_ok();
    
    if json_output {
        let output = json!({
            "status": "success",
            "asset_id": id,
            "valid": is_valid,
            "content_hash": hex::encode(asset.content_hash),
            "issuer_pubkey": hex::encode(asset.metadata.issuer_pubkey),
            "network": asset.network.to_string(),
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        if is_valid {
            println!("✅ Asset {} is valid!", id);
            println!("   Type: {}", asset.metadata.asset_type);
            println!("   Size: {} bytes", asset.content_size);
            println!("   Network: {}", asset.network);
        } else {
            println!("❌ Asset {} verification failed!", id);
            std::process::exit(1); // Non-zero exit code for automation
        }
    }
    
    Ok(())
}

/// Handle encrypted asset backup for data survival
pub async fn handle_backup(password: Option<String>, output_path: Option<String>, json_output: bool) -> Result<()> {
    info!("Starting encrypted asset backup");
    
    // Get backup password from user if not provided
    let backup_password = match password {
        Some(pwd) => pwd,
        None => {
            error!("Backup password required. Use --password or set BACKUP_PASSWORD env var");
            return Err(anyhow::anyhow!("Backup password required"));
        }
    };
    
    // Verify password strength
    if backup_password.len() < 12 {
        error!("Password too weak. Minimum 12 characters required");
        return Err(anyhow::anyhow!("Password too weak"));
    }
    
    // Open storage
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let storage = Storage::open_disk(data_dir.clone(), NetworkMode::Devnet).await
        .map_err(|e| anyhow::anyhow!("Failed to open storage: {}", e))?;
    
    // Get all assets
    let assets = storage.list_assets(10000).await
        .map_err(|e| anyhow::anyhow!("Failed to list assets for backup: {}", e))?;
    
    if assets.is_empty() {
        info!("No assets to backup");
        if json_output {
            let output = json!({
                "status": "success",
                "message": "No assets to backup",
                "asset_count": 0,
                "timestamp": chrono::Utc::now().timestamp()
            });
            println!("{}", output);
        } else {
            println!("ℹ️  No assets found to backup");
        }
        return Ok(());
    }
    
    // Create backup data structure
    let backup_data = json!({
        "version": "1.0",
        "created_at": chrono::Utc::now().timestamp(),
        "network": "devnet",
        "asset_count": assets.len(),
        "assets": assets.iter().map(|asset| {
            json!({
                "id": asset.id.to_string(),
                "asset_type": asset.metadata.asset_type.as_str(),
                "description": asset.metadata.description,
                "content_hash": hex::encode(asset.content_hash),
                "content_size": asset.content_size,
                "issuer_pubkey": hex::encode(asset.metadata.issuer_pubkey),
                "signature": hex::encode(asset.signature),
                "created_at": asset.metadata.created_at,
                "network": asset.network.to_string()
            })
        }).collect::<Vec<_>>()
    });
    
    // Serialize backup data
    let backup_json = backup_data.to_string();
    let backup_bytes = backup_json.as_bytes();
    
    // Derive encryption key from password using Argon2
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = argon2.hash_password(backup_password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to derive encryption key: {}", e))?;
    
    // Extract key from hash (simplified for demo - in production use proper KDF)
    // 1. Extraemos el hash y lo guardamos en una variable propia
    // Esto hace que el valor sea el dueño de los datos y no muera al final de la línea
    let hash_output = password_hash.hash.expect("Hash should not be empty");

    // 2. Ahora sí podemos tomar los bytes con seguridad
    let password_hash_bytes = hash_output.as_bytes();

    // 3. Tomamos los primeros 32 bytes y creamos la clave
    let key = Key::from_slice(&password_hash_bytes[..32]);
    let cipher = ChaCha20Poly1305::new(key);
    
    // Generate nonce
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    
    // Encrypt backup data
    let ciphertext = cipher.encrypt(&nonce, backup_bytes)
        .map_err(|e| anyhow::anyhow!("Failed to encrypt backup: {}", e))?;
    
    // Create backup package
    let backup_package = json!({
        "version": "1.0",
        "encryption": "chacha20poly1305",
        "kdf": "argon2",
        "salt": salt.as_str(),
        "nonce": hex::encode(nonce),
        "ciphertext": general_purpose::STANDARD.encode(&ciphertext),
        "created_at": chrono::Utc::now().timestamp(),
        "asset_count": assets.len(),
        "network": "devnet"
    });
    
    // Determine output path
    let backup_path = match output_path {
        Some(path) => PathBuf::from(path),
        None => {
            let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
            data_dir.join(format!("piva_backup_{}.json", timestamp))
        }
    };
    
    // Write backup file
    let backup_content = backup_package.to_string();
    fs::write(&backup_path, backup_content)
        .map_err(|e| anyhow::anyhow!("Failed to write backup file: {}", e))?;
    
    info!("Backup created successfully: {}", backup_path.display());
    
    if json_output {
        let output = json!({
            "status": "success",
            "backup_file": backup_path.to_string_lossy(),
            "asset_count": assets.len(),
            "encryption": "chacha20poly1305",
            "created_at": chrono::Utc::now().timestamp(),
            "file_size_bytes": fs::metadata(&backup_path)?.len()
        });
        println!("{}", output);
    } else {
        println!("✅ Encrypted backup created successfully!");
        println!("   Backup file: {}", backup_path.display());
        println!("   Assets backed up: {}", assets.len());
        println!("   Encryption: ChaCha20-Poly1305 + Argon2");
        println!("   File size: {} bytes", fs::metadata(&backup_path)?.len());
        println!("   ⚠️  Store this backup file securely with your password!");
    }
    
    Ok(())
}
