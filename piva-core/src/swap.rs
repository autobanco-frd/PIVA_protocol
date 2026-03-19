//! Hashed Time-Locked Contracts (HTLC) Module
//! 
//! Implements atomic swap logic for cross-chain and peer-to-peer exchanges.
//! This is the core component that prevents corruption and ensures fair exchange.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use sha2::{Sha256, Digest};
use piva_crypto::hash_sha3_256;

/// HTLC Contract Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcContract {
    /// Unique contract identifier
    pub contract_id: String,
    
    /// Asset being offered (e.g., PIVA asset ID)
    pub offer_asset: String,
    
    /// Asset being requested (e.g., BTC address, EVM address)
    pub request_asset: String,
    
    /// SHA-256 hash of the secret
    pub secret_hash: [u8; 32],
    
    /// Timeout timestamp (Unix timestamp in seconds)
    pub timeout: u64,
    
    /// Amount being offered
    pub offer_amount: u64,
    
    /// Amount being requested
    pub request_amount: u64,
    
    /// Contract creator's public key
    pub creator_pubkey: [u8; 32],
    
    /// Contract participant's public key (optional during creation)
    pub participant_pubkey: Option<[u8; 32]>,
    
    /// Contract status
    pub status: HtlcStatus,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Network mode
    pub network: String,
}

/// HTLC Contract Status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HtlcStatus {
    /// Contract created, waiting for participation
    Created,
    /// Contract funded by both parties, waiting for secret
    Funded,
    /// Contract completed successfully
    Completed,
    /// Contract timed out, funds returned
    Timeout,
    /// Contract cancelled
    Cancelled,
}

/// HTLC Manager for contract operations
pub struct HtlcManager {
    network: String,
}

impl HtlcManager {
    /// Create new HTLC manager
    pub fn new(network: &str) -> Self {
        Self {
            network: network.to_string(),
        }
    }
    
    /// Create a new HTLC contract
    pub fn create_contract(
        &self,
        offer_asset: String,
        request_asset: String,
        secret: &[u8], // Secret to be revealed later
        timeout_seconds: u64,
        offer_amount: u64,
        request_amount: u64,
        creator_pubkey: [u8; 32],
    ) -> Result<HtlcContract> {
        // Generate secret hash
        let secret_hash = hash_sha3_256(secret);
        
        // Generate contract ID
        let contract_id = self.generate_contract_id(&secret_hash, &creator_pubkey);
        
        // Calculate timeout
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let timeout = now + timeout_seconds;
        
        let contract = HtlcContract {
            contract_id,
            offer_asset,
            request_asset,
            secret_hash,
            timeout,
            offer_amount,
            request_amount,
            creator_pubkey,
            participant_pubkey: None,
            status: HtlcStatus::Created,
            created_at: now,
            network: self.network.clone(),
        };
        
        Ok(contract)
    }
    
    /// Participate in an existing HTLC contract
    pub fn participate_contract(
        &self,
        contract: &mut HtlcContract,
        participant_pubkey: [u8; 32],
    ) -> Result<()> {
        // Verify contract is in Created status
        if contract.status != HtlcStatus::Created {
            return Err(anyhow::anyhow!("Contract is not in Created status"));
        }
        
        // Update contract
        contract.participant_pubkey = Some(participant_pubkey);
        contract.status = HtlcStatus::Funded;
        
        Ok(())
    }
    
    /// Complete HTLC contract by revealing the secret
    pub fn complete_contract(
        &self,
        contract: &mut HtlcContract,
        secret: &[u8],
    ) -> Result<()> {
        // Verify contract is in Funded status
        if contract.status != HtlcStatus::Funded {
            return Err(anyhow::anyhow!("Contract is not in Funded status"));
        }
        
        // Verify secret matches hash
        let secret_hash = hash_sha3_256(secret);
        if secret_hash != contract.secret_hash {
            return Err(anyhow::anyhow!("Secret does not match hash"));
        }
        
        // Check for timeout
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > contract.timeout {
            return Err(anyhow::anyhow!("Contract has timed out"));
        }
        
        // Complete contract
        contract.status = HtlcStatus::Completed;
        
        Ok(())
    }
    
    /// Timeout HTLC contract (refund)
    pub fn timeout_contract(&self, contract: &mut HtlcContract) -> Result<()> {
        // Verify contract is in Funded status
        if contract.status != HtlcStatus::Funded {
            return Err(anyhow::anyhow!("Contract is not in Funded status"));
        }
        
        // Check for timeout
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now <= contract.timeout {
            return Err(anyhow::anyhow!("Contract has not timed out yet"));
        }
        
        // Timeout contract
        contract.status = HtlcStatus::Timeout;
        
        Ok(())
    }
    
    /// Generate contract ID from secret hash and creator pubkey
    fn generate_contract_id(&self, secret_hash: &[u8; 32], creator_pubkey: &[u8; 32]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret_hash);
        hasher.update(creator_pubkey);
        hasher.update(self.network.as_bytes());
        let hash = hasher.finalize();
        
        format!("htlc_{}", hex::encode(hash))
    }
    
    /// Verify contract integrity
    pub fn verify_contract(&self, contract: &HtlcContract) -> Result<bool> {
        // Verify contract ID
        let expected_id = self.generate_contract_id(&contract.secret_hash, &contract.creator_pubkey);
        if expected_id != contract.contract_id {
            return Ok(false);
        }
        
        // Verify timeout is in the future
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if contract.created_at > now || contract.timeout <= contract.created_at {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Get contract status with timeout check
    pub fn get_contract_status(&self, contract: &HtlcContract) -> HtlcStatus {
        // If contract is already completed or cancelled, return status
        if matches!(contract.status, HtlcStatus::Completed | HtlcStatus::Timeout | HtlcStatus::Cancelled) {
            return contract.status.clone();
        }
        
        // Check for timeout
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now > contract.timeout && contract.status == HtlcStatus::Funded {
            return HtlcStatus::Timeout;
        }
        
        contract.status.clone()
    }
}

/// HTLC Secret Generator
pub struct HtlcSecret;

impl HtlcSecret {
    /// Generate a new random secret
    pub fn generate() -> [u8; 32] {
        use rand::RngCore;
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        secret
    }
    
    /// Generate secret from string
    pub fn from_string(secret_str: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(secret_str.as_bytes());
        let hash = hasher.finalize();
        
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&hash);
        secret
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_htlc_contract_creation() {
        let manager = HtlcManager::new("devnet");
        let secret = HtlcSecret::generate();
        let creator_pubkey = [1u8; 32];
        
        let contract = manager.create_contract(
            "asset_123".to_string(),
            "btc_address_abc".to_string(),
            &secret,
            3600, // 1 hour timeout
            1000,
            50000,
            creator_pubkey,
        ).unwrap();
        
        assert_eq!(contract.status, HtlcStatus::Created);
        assert_eq!(contract.offer_asset, "asset_123");
        assert_eq!(contract.request_asset, "btc_address_abc");
        assert_eq!(contract.offer_amount, 1000);
        assert_eq!(contract.request_amount, 50000);
    }
    
    #[test]
    fn test_htlc_contract_completion() {
        let manager = HtlcManager::new("devnet");
        let secret = HtlcSecret::generate();
        let creator_pubkey = [1u8; 32];
        let participant_pubkey = [2u8; 32];
        
        let mut contract = manager.create_contract(
            "asset_123".to_string(),
            "btc_address_abc".to_string(),
            &secret,
            3600,
            1000,
            50000,
            creator_pubkey,
        ).unwrap();
        
        // Participate
        manager.participate_contract(&mut contract, participant_pubkey).unwrap();
        assert_eq!(contract.status, HtlcStatus::Funded);
        
        // Complete
        manager.complete_contract(&mut contract, &secret).unwrap();
        assert_eq!(contract.status, HtlcStatus::Completed);
    }
    
    #[test]
    fn test_htlc_contract_timeout() {
        let manager = HtlcManager::new("devnet");
        let secret = HtlcSecret::generate();
        let creator_pubkey = [1u8; 32];
        let participant_pubkey = [2u8; 32];
        
        let mut contract = manager.create_contract(
            "asset_123".to_string(),
            "btc_address_abc".to_string(),
            &secret,
            1, // 1 second timeout
            1000,
            50000,
            creator_pubkey,
        ).unwrap();
        
        // Participate
        manager.participate_contract(&mut contract, participant_pubkey).unwrap();
        
        // Wait for timeout
        std::thread::sleep(std::time::Duration::from_secs(2));
        
        // Timeout
        manager.timeout_contract(&mut contract).unwrap();
        assert_eq!(contract.status, HtlcStatus::Timeout);
    }
}
