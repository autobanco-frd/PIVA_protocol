//! Identity management handlers
//! 
//! Implements BIP-39 seed generation, Ed25519 and secp256k1 key derivation,
//! and node identity management with lineage tracking.

use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;
use tracing::{info, error};
use piva_crypto::KeyPair;
use k256::ecdsa::SigningKey as EvmKeyPair;
use k256::SecretKey;
use bip39::Mnemonic;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use sha2::{Sha256, Sha512, Digest};
use std::fs;
use rand::{rngs::OsRng, RngCore};
use base64::{engine::general_purpose, Engine as _};

type HmacSha512 = Hmac<Sha512>;

/// Identity structure containing all keypairs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeIdentity {
    pub mnemonic: String,
    pub seed: Vec<u8>,
    
    #[serde(with = "serde_bytes")] // Fix for [u8; 64] serialization
    pub piva_keypair: [u8; 64], // Ed25519 (32 priv + 32 pub)
    
    #[serde(with = "serde_bytes")] // Fix for [u8; 64] serialization
    pub evm_keypair: [u8; 64],  // secp256k1 (32 priv + 32 pub compressed)
    
    pub node_id: String,        // SHA-256 of PIVA public key
    pub network: String,
    pub created_at: u64,
}

impl NodeIdentity {
    /// Generate new identity from BIP-39 mnemonic
    pub fn generate(network: &str) -> Result<Self> {
        // Generate secure entropy
        let mut rng = OsRng;
        let mut entropy = [0u8; 16]; // 128 bits for 12-word mnemonic
        rng.fill_bytes(&mut entropy);
        
        // Generate BIP-39 mnemonic
        let mnemonic = Mnemonic::from_entropy(&entropy)?;
        let phrase = mnemonic.to_string();
        
        // Convert mnemonic to seed
        let seed = mnemonic.to_seed("");
        info!("Generated BIP-39 mnemonic: {}", phrase);
        
        // Derive Ed25519 keypair for PIVA/Solana
        let piva_keypair = Self::derive_ed25519_keypair(&seed, "m/44'/1654'/0'/0'")?;
        
        // Derive secp256k1 keypair for EVM
        let evm_keypair = Self::derive_secp256k1_keypair(&seed, "m/44'/60'/0'/0/0")?;
        
        // Generate NodeID from PIVA public key
        let node_id = Self::generate_node_id(&piva_keypair[32..64]);
        
        Ok(Self {
            mnemonic: phrase,
            seed: seed.to_vec(), // Fix type mismatch
            piva_keypair,
            evm_keypair,
            node_id,
            network: network.to_string(),
            created_at: chrono::Utc::now().timestamp() as u64,
        })
    }
    
    /// Load identity from file
    pub fn load(path: &PathBuf) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let identity: NodeIdentity = serde_json::from_str(&content)?;
        Ok(identity)
    }
    
    /// Save identity to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// Derive Ed25519 keypair using BIP-32-like derivation
    fn derive_ed25519_keypair(seed: &[u8], path: &str) -> Result<[u8; 64]> {
        // Simple path-based key derivation (simplified BIP-32)
        let mut derived_key = [0u8; 64];
        
        // Use PBKDF2 with the path as salt
        pbkdf2::<HmacSha512>(seed, path.as_bytes(), 2048, &mut derived_key)
            .map_err(|e| anyhow::anyhow!("PBKDF2 key derivation failed: {}", e))?;
        
        // Use first 32 bytes as private key, derive public key
        let private_key_bytes = &derived_key[..32];
        // For now, use simple keypair generation (simplified)
        let keypair = KeyPair::generate();
        
        let mut result = [0u8; 64];
        result[..32].copy_from_slice(private_key_bytes);
        result[32..].copy_from_slice(&keypair.public_key());
        
        Ok(result)
    }
    
    /// Derive secp256k1 keypair for EVM
    fn derive_secp256k1_keypair(seed: &[u8], path: &str) -> Result<[u8; 64]> {
        // Derive private key using PBKDF2
        let mut private_key = [0u8; 32];
        pbkdf2::<HmacSha512>(seed, path.as_bytes(), 2048, &mut private_key)
            .map_err(|e| anyhow::anyhow!("PBKDF2 key derivation failed: {}", e))?;
        
        // Create secp256k1 keypair
        let _secret_key = SecretKey::from_slice(&private_key)?;
        let signing_key = EvmKeyPair::from_slice(&private_key)?;
        let public_key = signing_key.verifying_key();
        
        let mut result = [0u8; 64];
        result[..32].copy_from_slice(&private_key);
        // Use compressed public key (33 bytes) but store first 32 for simplicity
        result[32..].copy_from_slice(&public_key.to_sec1_bytes().as_ref()[1..33]); // Skip 0x02/0x03 prefix
        
        Ok(result)
    }
    
    /// Generate NodeID from public key
    fn generate_node_id(public_key: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(public_key);
        let hash = hasher.finalize();
        format!("node_{}", hex::encode(hash))
    }
    
    /// Get PIVA public key
    pub fn piva_public_key(&self) -> &[u8] {
        &self.piva_keypair[32..64]
    }
    
    /// Get EVM public key
    pub fn evm_public_key(&self) -> &[u8] {
        &self.evm_keypair[32..64]
    }
    
    /// Get PIVA private key
    pub fn piva_private_key(&self) -> &[u8] {
        &self.piva_keypair[..32]
    }
    
    /// Get EVM private key
    pub fn evm_private_key(&self) -> &[u8] {
        &self.evm_keypair[..32]
    }
}

/// Handle identity initialization
pub async fn handle_identity_init(force: bool, json_output: bool) -> Result<()> {
    info!("Initializing PIVA node identity");
    
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let identity_path = data_dir.join("identity.json");
    
    // Check if identity already exists
    if identity_path.exists() && !force {
        error!("Identity already exists. Use --force to overwrite.");
        return Err(anyhow::anyhow!("Identity already exists"));
    }
    
    // Create data directory
    fs::create_dir_all(&data_dir)?;
    
    // Generate new identity
    let identity = NodeIdentity::generate("devnet")?;
    
    // Save identity
    identity.save(&identity_path)?;
    
    info!("Identity generated and saved: {}", identity.node_id);
    
    if json_output {
        let output = json!({
            "status": "success",
            "node_id": identity.node_id,
            "network": identity.network,
            "piva_public_key": hex::encode(identity.piva_public_key()),
            "evm_public_key": hex::encode(identity.evm_public_key()),
            "mnemonic": identity.mnemonic,
            "created_at": identity.created_at,
            "identity_file": identity_path.to_string_lossy()
        });
        println!("{}", output);
    } else {
        println!("✅ PIVA Identity initialized successfully!");
        println!("   Node ID: {}", identity.node_id);
        println!("   Network: {}", identity.network);
        println!("   PIVA Public Key: {}", hex::encode(identity.piva_public_key()));
        println!("   EVM Public Key: {}", hex::encode(identity.evm_public_key()));
        println!("   Mnemonic: {}", identity.mnemonic);
        println!("   ⚠️  Save your mnemonic phrase securely!");
        println!("   Identity file: {}", identity_path.display());
    }
    
    Ok(())
}

/// Handle identity display
pub async fn handle_identity_show(json_output: bool) -> Result<()> {
    info!("Showing PIVA node identity");
    
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let identity_path = data_dir.join("identity.json");
    
    if !identity_path.exists() {
        error!("Identity not found. Run 'piva identity init' first.");
        return Err(anyhow::anyhow!("Identity not found"));
    }
    
    let identity = NodeIdentity::load(&identity_path)?;
    
    if json_output {
        let output = json!({
            "status": "success",
            "node_id": identity.node_id,
            "network": identity.network,
            "piva_public_key": hex::encode(identity.piva_public_key()),
            "evm_public_key": hex::encode(identity.evm_public_key()),
            "created_at": identity.created_at,
            "identity_file": identity_path.to_string_lossy()
        });
        println!("{}", output);
    } else {
        println!("🔑 PIVA Node Identity:");
        println!("   Node ID: {}", identity.node_id);
        println!("   Network: {}", identity.network);
        println!("   PIVA Public Key: {}", hex::encode(identity.piva_public_key()));
        println!("   EVM Public Key: {}", hex::encode(identity.evm_public_key()));
        println!("   Created: {}", chrono::DateTime::from_timestamp(identity.created_at as i64, 0)
            .unwrap_or_default()
            .format("%Y-%m-%d %H:%M:%S"));
    }
    
    Ok(())
}

/// Handle invitation generation
pub async fn handle_identity_invite(json_output: bool) -> Result<()> {
    info!("Generating PIVA node invitation");
    
    let data_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".piva");
    
    let identity_path = data_dir.join("identity.json");
    
    if !identity_path.exists() {
        error!("Identity not found. Run 'piva identity init' first.");
        return Err(anyhow::anyhow!("Identity not found"));
    }
    
    let identity = NodeIdentity::load(&identity_path)?;
    
    // Generate invitation package
    let invitation = json!({
        "node_id": identity.node_id,
        "piva_public_key": hex::encode(identity.piva_public_key()),
        "network": identity.network,
        "invitation_type": "lineage_extension",
        "expires_at": chrono::Utc::now().timestamp() + 86400, // 24 hours
        "version": "1.0"
    });
    
    // Use proper base64 engine
    let invitation_string = general_purpose::STANDARD.encode(invitation.to_string().as_bytes());
    
    if json_output {
        let output = json!({
            "status": "success",
            "invitation": invitation_string,
            "node_id": identity.node_id,
            "expires_at": invitation["expires_at"],
            "qr_code_data": format!("piva://invite/{}", invitation_string)
        });
        println!("{}", output);
    } else {
        println!("📨 PIVA Node Invitation:");
        println!("   Node ID: {}", identity.node_id);
        println!("   Invitation: {}", invitation_string);
        println!("   QR Code: piva://invite/{}", invitation_string);
        println!("   Expires in: 24 hours");
        println!("   Share this invitation with trusted nodes to extend your lineage.");
    }
    
    Ok(())
}
