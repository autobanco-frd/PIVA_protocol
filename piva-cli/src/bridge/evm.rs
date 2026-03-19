//! EVM Bridge Handler
//! 
//! Implements EVM interoperability using alloy-sol-types for calldata generation
//! and secp256k1 for signing. This enables PIVA assets to be represented
//! on Ethereum/EVM compatible chains.

use anyhow::Result;
use serde_json::json;
use tracing::info;
use alloy_sol_types::{sol, SolCall};
use alloy_primitives::{Address, U256, FixedBytes, Bytes};
use alloy_signer_local::LocalSigner;
use k256::ecdsa::SigningKey;
use std::str::FromStr;
use sha2::{Sha256, Digest};

// Define the PIVA Bridge contract interface using sol! macro
sol! {
    /// PIVA Bridge Contract Interface
    interface PIVABridge {
        /// Bridge a PIVA asset to EVM chain
        function bridgeAsset(
            bytes32 assetHash,
            bytes32 assetId,
            address recipient,
            uint256 amount,
            bytes signature
        ) external returns (uint256 bridgeId);
        
        /// Get bridged asset info
        function getBridgedAsset(uint256 bridgeId) external view returns (
            bytes32 assetHash,
            bytes32 assetId,
            address recipient,
            uint256 amount,
            bool completed
        );
        
        /// Complete bridge (reveal secret for HTLC)
        function completeBridge(
            uint256 bridgeId,
            bytes32 secret
        ) external;
    }
}

/// EVM Bridge Manager
pub struct EvmBridgeManager {
    network: String,
    contract_address: Address,
}

impl EvmBridgeManager {
    /// Create new EVM bridge manager
    pub fn new(network: &str, contract_address: &str) -> Result<Self> {
        let address = Address::from_str(contract_address)
            .map_err(|e| anyhow::anyhow!("Invalid contract address: {}", e))?;
        
        Ok(Self {
            network: network.to_string(),
            contract_address: address,
        })
    }
    
    /// Generate calldata for bridging asset
    pub fn encode_bridge_asset(
        &self,
        asset_hash: &[u8; 32],
        asset_id: &str,
        recipient: &str,
        amount: u64,
        signature: &[u8; 64], // Ed25519 signature
    ) -> Result<String> {
        // Convert inputs to proper types
        let asset_hash_bytes: FixedBytes<32> = FixedBytes::from_slice(asset_hash);
        
        // For asset_id, use first 32 bytes of hash if longer than 32
        let asset_id_bytes = if asset_id.len() <= 32 {
            FixedBytes::from_slice(asset_id.as_bytes())
        } else {
            // Hash the asset_id if it's too long
            let id_hash = sha2::Sha256::digest(asset_id.as_bytes());
            FixedBytes::from_slice(&id_hash[..32])
        };
        
        let recipient_address = Address::from_str(recipient)
            .map_err(|e| anyhow::anyhow!("Invalid recipient address: {}", e))?;
        let amount_uint = U256::from(amount);
        let signature_bytes = Bytes::copy_from_slice(signature);
        
        // Create the call data
        let call = PIVABridge::bridgeAssetCall {
            assetHash: asset_hash_bytes,
            assetId: asset_id_bytes,
            recipient: recipient_address,
            amount: amount_uint,
            signature: signature_bytes,
        };
        
        // Encode to calldata
        let calldata = call.abi_encode();
        Ok(format!("0x{}", hex::encode(calldata)))
    }
    
    /// Generate calldata for completing bridge (HTLC)
    pub fn encode_complete_bridge(
        &self,
        bridge_id: u64,
        secret: &[u8; 32],
    ) -> Result<String> {
        let bridge_id_uint = U256::from(bridge_id);
        let secret_bytes: FixedBytes<32> = FixedBytes::from_slice(secret);
        
        let call = PIVABridge::completeBridgeCall {
            bridgeId: bridge_id_uint,
            secret: secret_bytes,
        };
        
        let calldata = call.abi_encode();
        Ok(format!("0x{}", hex::encode(calldata)))
    }
    
    /// Sign calldata with secp256k1 private key
    pub fn sign_calldata(
        &self,
        calldata: &str,
        private_key: &[u8; 32],
    ) -> Result<String> {
        // Remove 0x prefix if present
        let calldata_clean = calldata.strip_prefix("0x").unwrap_or(calldata);
        
        // Decode calldata
        let calldata_bytes = hex::decode(calldata_clean)
            .map_err(|e| anyhow::anyhow!("Invalid calldata hex: {}", e))?;
        
        // Create signing key
        let signing_key = SigningKey::from_slice(private_key)
            .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
        
        // Sign the calldata hash (simplified - in production use proper EIP-191 signing)
        let mut hasher = Sha256::new();
        hasher.update(&calldata_bytes);
        let hash = hasher.finalize();
        
        let signature = signing_key.sign_prehash_recoverable(&hash[..])
            .map_err(|e| anyhow::anyhow!("Failed to sign calldata: {}", e))?;
        
        // Convert signature to hex string (65 bytes: r + s + v)
        // signature.0 is the signature, signature.1 is the recovery ID (v)
        let mut sig_bytes = signature.0.to_vec();
        sig_bytes.push(signature.1.to_byte()); // Add recovery byte for EVM compatibility
        Ok(format!("0x{}", hex::encode(sig_bytes)))
    }
    
    /// Verify EVM address matches secp256k1 public key
    pub fn verify_address(
        &self,
        expected_address: &str,
        public_key: &[u8; 32],
    ) -> Result<bool> {
        // Convert public key to secp256k1 format
        let signing_key = SigningKey::from_slice(public_key)
            .map_err(|_| anyhow::anyhow!("Invalid public key for secp256k1"))?;
        
        // Derive address from public key
        let signer = LocalSigner::from_signing_key(signing_key);
        let derived_address = signer.address();
        
        let expected_addr = Address::from_str(expected_address)
            .map_err(|e| anyhow::anyhow!("Invalid expected address: {}", e))?;
        
        // Accedemos al contenido interno y lo convertimos en el array de bytes puro [u8; 20]
        let addr_array: [u8; 20] = derived_address.0.into(); 

        // Ahora comparamos convirtiendo ese array a Address
        Ok(Address::from(addr_array) == expected_addr)
    }
}

/// Handle EVM bridge encoding
pub async fn handle_evm_encode(
    asset_id: String,
    dry_run: bool,
    json_output: bool,
) -> Result<()> {
    info!("EVM bridge encoding for asset: {}", asset_id);
    
    // Parse asset ID
    let asset_id_parsed = asset_id.parse::<piva_core::asset::AssetId>()
        .map_err(|e| anyhow::anyhow!("Invalid asset ID: {}", e))?;
    
    // Create bridge manager (using placeholder contract address)
    let bridge = EvmBridgeManager::new("devnet", "0x1234567890123456789012345678901234567890")?;
    
    // For demo, generate placeholder values
    let asset_hash = [1u8; 32]; // Would come from asset storage
    let recipient = "0x1234567890123456789012345678901234567890";
    let amount = 1000;
    let signature = [2u8; 64]; // Would come from asset signature
    
    // Generate calldata
    let calldata = bridge.encode_bridge_asset(
        &asset_hash,
        &asset_id_parsed.to_string(),
        recipient,
        amount,
        &signature,
    )?;
    
    if json_output {
        let output = json!({
            "status": "success",
            "asset_id": asset_id,
            "calldata": calldata,
            "contract_address": bridge.contract_address.to_string(),
            "network": bridge.network,
            "dry_run": dry_run,
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("🌐 EVM Bridge Calldata Generated:");
        println!("   Asset ID: {}", asset_id);
        println!("   Contract: {}", bridge.contract_address);
        println!("   Recipient: {}", recipient);
        println!("   Amount: {}", amount);
        println!("   Calldata: {}", calldata);
        
        if dry_run {
            println!("   🔇 DRY RUN - No transaction sent");
        } else {
            println!("   ⚠️  Ready to send to EVM network");
        }
    }
    
    Ok(())
}

/// Handle EVM bridge signing
pub async fn handle_evm_sign(
    calldata: String,
    json_output: bool,
) -> Result<()> {
    info!("EVM bridge signing for calldata");
    
    // For demo, generate ephemeral private key
    // In production, this would come from identity storage
    let mut rng = rand::rngs::OsRng;
    let private_key = k256::ecdsa::SigningKey::random(&mut rng)
        .to_bytes()
        .into();
    
    // Create bridge manager
    let bridge = EvmBridgeManager::new("devnet", "0x1234567890123456789012345678901234567890")?;
    
    // Sign calldata
    let signature = bridge.sign_calldata(&calldata, &private_key)?;
    
    if json_output {
        let output = json!({
            "status": "success",
            "calldata": calldata,
            "signature": signature,
            "signer_address": "0x1234567890123456789012345678901234567890", // Would be derived
            "network": bridge.network,
            "timestamp": chrono::Utc::now().timestamp()
        });
        println!("{}", output);
    } else {
        println!("✍️  EVM Transaction Signed:");
        println!("   Calldata: {}", calldata);
        println!("   Signature: {}", signature);
        println!("   Network: {}", bridge.network);
        println!("   Ready to broadcast to EVM network");
    }
    
    Ok(())
}
