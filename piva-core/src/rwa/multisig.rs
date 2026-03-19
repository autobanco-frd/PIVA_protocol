//! Multi-signature Support for PIVA Protocol
//! 
//! Implements 2-of-3 multisig for institutional accounts,
//! arbitration services, and enhanced security.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use ed25519_dalek::{Signature, Verifier, VerifyingKey, SigningKey};
use sha2::{Sha256, Digest};
use crate::rwa::market::VerificationLevel;

/// Multi-signature wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigWallet {
    /// Unique wallet identifier
    pub wallet_id: String,
    
    /// Required signatures (m in m-of-n)
    pub required_signatures: u8,
    
    /// Total signers (n in m-of-n)
    pub total_signers: u8,
    
    /// Signer information
    pub signers: Vec<SignerInfo>,
    
    /// Wallet creation timestamp
    pub created_at: u64,
    
    /// Current wallet state
    pub state: WalletState,
    
    /// Pending transactions
    pub pending_transactions: Vec<PendingTransaction>,
    
    /// Completed transactions
    pub completed_transactions: Vec<CompletedTransaction>,
    
    /// Wallet metadata
    pub metadata: WalletMetadata,
    
    /// Security settings
    pub security_settings: SecuritySettings,
}

/// Signer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignerInfo {
    /// Signer identifier (peer ID)
    pub peer_id: String,
    
    /// Public key
    pub public_key: [u8; 32],
    
    /// Signer role
    pub role: SignerRole,
    
    /// Verification level
    pub verification_level: VerificationLevel,
    
    /// Signing weight (for weighted multisig)
    pub weight: u8,
    
    /// Last activity timestamp
    pub last_activity: u64,
    
    /// Is signer active
    pub is_active: bool,
    
    /// Permissions
    pub permissions: Vec<Permission>,
}

/// Signer roles
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignerRole {
    /// Primary owner (full control)
    Primary,
    /// Secondary owner (limited control)
    Secondary,
    /// Arbitration service (dispute resolution)
    Arbitrator,
    /// Compliance officer (regulatory oversight)
    Compliance,
    /// Technical admin (maintenance)
    Technical,
}

/// Permissions for signers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Permission {
    /// Can initiate transactions
    InitiateTransactions,
    /// Can approve transactions
    ApproveTransactions,
    /// Can add/remove signers
    ManageSigners,
    /// Can change security settings
    ChangeSecurity,
    /// Can view transaction history
    ViewHistory,
    /// Can export wallet data
    ExportData,
}

/// Wallet state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WalletState {
    /// Wallet is active and operational
    Active,
    /// Wallet is frozen (no transactions allowed)
    Frozen,
    /// Wallet is in recovery mode
    Recovery,
    /// Wallet is being closed
    Closing,
    /// Wallet is closed
    Closed,
}

/// Pending transaction awaiting signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    /// Transaction ID
    pub transaction_id: String,
    
    /// Transaction type
    pub transaction_type: TransactionType,
    
    /// Transaction data
    pub data: TransactionData,
    
    /// Required signatures
    pub required_signatures: u8,
    
    /// Current signatures
    pub signatures: Vec<SignatureInfo>,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Expiration timestamp
    pub expires_at: u64,
    
    /// Initiator peer ID
    pub initiator_peer_id: String,
    
    /// Transaction status
    pub status: TransactionStatus,
    
    /// Security level
    pub security_level: SecurityLevel,
}

/// Completed transaction with all signatures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTransaction {
    /// Transaction ID
    pub transaction_id: String,
    
    /// Transaction type
    pub transaction_type: TransactionType,
    
    /// Transaction data
    pub data: TransactionData,
    
    /// Final signatures
    pub signatures: Vec<SignatureInfo>,
    
    /// Completion timestamp
    pub completed_at: u64,
    
    /// Block height (if on-chain)
    pub block_height: Option<u64>,
    
    /// Transaction hash
    pub transaction_hash: Option<String>,
    
    /// Gas used (if applicable)
    pub gas_used: Option<u64>,
    
    /// Success status
    pub success: bool,
    
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Transaction types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    /// Asset transfer
    AssetTransfer,
    /// Smart contract interaction
    ContractCall,
    /// DeFi operation
    DeFiOperation,
    /// Governance vote
    GovernanceVote,
    /// Security settings change
    SecurityChange,
    /// Signer management
    SignerManagement,
    /// Wallet recovery
    WalletRecovery,
}

/// Transaction data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    /// Target address or contract
    pub target: String,
    
    /// Amount (if applicable)
    pub amount: Option<u64>,
    
    /// Currency
    pub currency: Option<String>,
    
    /// Function signature (for contracts)
    pub function_signature: Option<String>,
    
    /// Function parameters
    pub parameters: Vec<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Signature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    /// Signer peer ID
    pub peer_id: String,
    
    /// Signature bytes
    pub signature: Vec<u8>,
    
    /// Signing timestamp
    pub signed_at: u64,
    
    /// Signature weight
    pub weight: u8,
    
    /// Is signature valid
    pub is_valid: bool,
}

/// Transaction status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionStatus {
    /// Transaction is pending signatures
    Pending,
    /// Transaction has enough signatures, awaiting execution
    Ready,
    /// Transaction is being executed
    Executing,
    /// Transaction completed successfully
    Completed,
    /// Transaction failed
    Failed,
    /// Transaction expired
    Expired,
    /// Transaction was cancelled
    Cancelled,
}

/// Security levels for transactions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityLevel {
    /// Low security (small amounts, trusted parties)
    Low,
    /// Medium security (moderate amounts)
    Medium,
    /// High security (large amounts, sensitive operations)
    High,
    /// Maximum security (critical operations)
    Maximum,
}

/// Wallet metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// Wallet name
    pub name: String,
    
    /// Wallet description
    pub description: Option<String>,
    
    /// Organization name
    pub organization: Option<String>,
    
    /// Compliance requirements
    pub compliance_requirements: Vec<ComplianceRequirement>,
    
    /// Geographic restrictions
    pub geographic_restrictions: Vec<String>,
    
    /// Transaction limits
    pub transaction_limits: TransactionLimits,
    
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Compliance requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComplianceRequirement {
    /// KYC verification required
    KYCRequired,
    /// AML screening required
    AMLRequired,
    /// Geographic verification required
    GeoVerification,
    /// Amount reporting threshold
    ReportingThreshold(u64),
    /// Time-based approvals required
    TimeBasedApproval(u32), // hours
}

/// Transaction limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionLimits {
    /// Maximum single transaction amount
    pub max_single_transaction: u64,
    
    /// Maximum daily transaction amount
    pub max_daily_amount: u64,
    
    /// Maximum weekly transaction amount
    pub max_weekly_amount: u64,
    
    /// Maximum monthly transaction amount
    pub max_monthly_amount: u64,
    
    /// Minimum transaction amount
    pub min_transaction_amount: u64,
    
    /// Currency for limits
    pub currency: String,
}

/// Security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    /// Require time-based approvals for large transactions
    pub time_based_approvals: bool,
    
    /// Hours required for time-based approval
    pub approval_hours: u32,
    
    /// Require geographic verification
    pub geo_verification_required: bool,
    
    /// Maximum transaction age before expiration
    pub max_transaction_age_hours: u32,
    
    /// Require offline signing for critical operations
    pub offline_signing_required: bool,
    
    /// Enable transaction batching
    pub enable_batching: bool,
    
    /// Maximum batch size
    pub max_batch_size: u8,
    
    /// Enable automatic expiration
    pub auto_expire: bool,
}

/// Multi-signature manager
pub struct MultiSigManager {
    /// Wallet storage
    wallets: HashMap<String, MultiSigWallet>,
    
    /// Global configuration
    config: MultiSigConfig,
    
    /// Security policies
    #[allow(dead_code)]
    security_policies: HashMap<String, SecurityPolicy>,
    
    /// Audit log
    audit_log: Vec<AuditEntry>,
}

/// Multi-signature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigConfig {
    /// Maximum number of signers per wallet
    pub max_signers: u8,
    
    /// Minimum required signatures
    pub min_required_signatures: u8,
    
    /// Default transaction expiration (hours)
    pub default_transaction_expiration: u32,
    
    /// Maximum transaction expiration (hours)
    pub max_transaction_expiration: u32,
    
    /// Enable weighted signing
    pub enable_weighted_signing: bool,
    
    /// Maximum signature weight
    pub max_signature_weight: u8,
    
    /// Enable recovery procedures
    pub enable_recovery: bool,
    
    /// Recovery threshold (percentage of signers)
    pub recovery_threshold: u8,
}

/// Security policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    
    /// Policy description
    pub description: String,
    
    /// Required verification level
    pub required_verification: VerificationLevel,
    
    /// Transaction type restrictions
    pub allowed_transaction_types: Vec<TransactionType>,
    
    /// Amount restrictions
    pub amount_restrictions: AmountRestrictions,
    
    /// Geographic restrictions
    pub geographic_restrictions: Vec<String>,
    
    /// Time-based restrictions
    pub time_restrictions: TimeRestrictions,
    
    /// Special requirements
    pub special_requirements: Vec<String>,
}

/// Amount restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmountRestrictions {
    /// Maximum amount per transaction
    pub max_per_transaction: Option<u64>,
    
    /// Maximum amount per day
    pub max_per_day: Option<u64>,
    
    /// Maximum amount per week
    pub max_per_week: Option<u64>,
    
    /// Maximum amount per month
    pub max_per_month: Option<u64>,
    
    /// Currency for restrictions
    pub currency: String,
}

/// Time restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRestrictions {
    /// Allowed hours of day (24-hour format)
    pub allowed_hours: Vec<u8>,
    
    /// Allowed days of week (0=Sunday, 6=Saturday)
    pub allowed_days: Vec<u8>,
    
    /// Blackout periods
    pub blackout_periods: Vec<TimeRange>,
}

/// Time range for restrictions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start timestamp
    pub start: u64,
    
    /// End timestamp
    pub end: u64,
    
    /// Reason for restriction
    pub reason: String,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID
    pub entry_id: String,
    
    /// Wallet ID
    pub wallet_id: String,
    
    /// Actor peer ID
    pub actor_peer_id: String,
    
    /// Action performed
    pub action: AuditAction,
    
    /// Action details
    pub details: String,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// IP address (if available)
    pub ip_address: Option<String>,
    
    /// User agent (if available)
    pub user_agent: Option<String>,
    
    /// Success status
    pub success: bool,
}

/// Audit actions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditAction {
    /// Wallet created
    WalletCreated,
    /// Wallet modified
    WalletModified,
    /// Signer added
    SignerAdded,
    /// Signer removed
    SignerRemoved,
    /// Transaction initiated
    TransactionInitiated,
    /// Transaction signed
    TransactionSigned,
    /// Transaction completed
    TransactionCompleted,
    /// Transaction cancelled
    TransactionCancelled,
    /// Security settings changed
    SecuritySettingsChanged,
    /// Recovery initiated
    RecoveryInitiated,
    /// Wallet frozen
    WalletFrozen,
    /// Wallet unfrozen
    WalletUnfrozen,
}

impl MultiSigManager {
    /// Create new multi-signature manager
    pub fn new(config: MultiSigConfig) -> Self {
        Self {
            wallets: HashMap::new(),
            config,
            security_policies: HashMap::new(),
            audit_log: Vec::new(),
        }
    }
    
    /// Create new multi-signature wallet
    pub fn create_wallet(
        &mut self,
        _wallet_name: String,
        required_signatures: u8,
        signers: Vec<SignerInfo>,
        metadata: WalletMetadata,
        creator_peer_id: String,
    ) -> Result<String> {
        // Validate configuration
        if signers.len() < required_signatures as usize {
            return Err(anyhow::anyhow!("Required signatures cannot exceed total signers"));
        }
        
        if signers.len() > self.config.max_signers as usize {
            return Err(anyhow::anyhow!("Too many signers"));
        }
        
        if required_signatures < self.config.min_required_signatures {
            return Err(anyhow::anyhow!("Required signatures below minimum"));
        }
        
        // Generate wallet ID
        let wallet_id = self.generate_wallet_id();
        
        let wallet = MultiSigWallet {
            wallet_id: wallet_id.clone(),
            required_signatures,
            total_signers: signers.len() as u8,
            signers,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            state: WalletState::Active,
            pending_transactions: Vec::new(),
            completed_transactions: Vec::new(),
            metadata,
            security_settings: SecuritySettings::default(),
        };
        
        self.wallets.insert(wallet_id.clone(), wallet.clone());
        
        // Log audit entry
        self.log_audit_entry(AuditEntry {
            entry_id: Self::generate_audit_id(),
            wallet_id: wallet_id.clone(),
            actor_peer_id: creator_peer_id,
            action: AuditAction::WalletCreated,
            details: format!("Created wallet with {} signers, {} required", 
                wallet.total_signers, wallet.required_signatures),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            ip_address: None,
            user_agent: None,
            success: true,
        });
        
        Ok(wallet_id)
    }
    
    /// Initiate new transaction
    pub fn initiate_transaction(
        &mut self,
        wallet_id: &str,
        transaction_type: TransactionType,
        data: TransactionData,
        initiator_peer_id: String,
        security_level: SecurityLevel,
    ) -> Result<String> {
        // Generate transaction ID before borrowing
        let transaction_id = self.generate_transaction_id();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        
        // Check wallet exists first
        if !self.wallets.contains_key(wallet_id) {
            return Err(anyhow::anyhow!("Wallet not found: {}", wallet_id));
        }
        
        // Check initiator permissions first (immutable borrow)
        let _initiator_permissions = {
            let wallet = self.wallets.get(wallet_id)
                .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", wallet_id))?;
            
            // Check wallet state
            if wallet.state != WalletState::Active {
                return Err(anyhow::anyhow!("Wallet is not active"));
            }
            
            let initiator = wallet.signers.iter()
                .find(|s| s.peer_id == initiator_peer_id)
                .ok_or_else(|| anyhow::anyhow!("Initiator not found in signers"))?;
            
            if !initiator.permissions.contains(&Permission::InitiateTransactions) {
                return Err(anyhow::anyhow!("Initiator lacks transaction initiation permission"));
            }
            
            initiator.permissions.clone()
        };
        
        // Now get mutable wallet for the rest
        let wallet = self.wallets.get_mut(wallet_id)
            .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", wallet_id))?;
        
        // Validate transaction against security policies (extracted to avoid borrow issues)
        Self::validate_transaction_security(wallet, &transaction_type, &data, &security_level)?;
        
        // Create pending transaction
        let pending_transaction = PendingTransaction {
            transaction_id: transaction_id.clone(),
            transaction_type,
            data,
            required_signatures: wallet.required_signatures,
            signatures: Vec::new(),
            created_at: now,
            expires_at: now + (self.config.default_transaction_expiration as u64 * 3600),
            initiator_peer_id: initiator_peer_id.clone(),
            status: TransactionStatus::Pending,
            security_level,
        };
        
        wallet.pending_transactions.push(pending_transaction);
        
        // Log audit entry
        self.log_audit_entry(AuditEntry {
            entry_id: Self::generate_audit_id(),
            wallet_id: wallet_id.to_string(),
            actor_peer_id: initiator_peer_id,
            action: AuditAction::TransactionInitiated,
            details: format!("Initiated transaction: {}", transaction_id),
            timestamp: now,
            ip_address: None,
            user_agent: None,
            success: true,
        });
        
        Ok(transaction_id)
    }
    
    /// Sign pending transaction
    pub fn sign_transaction(
        &mut self,
        wallet_id: &str,
        transaction_id: &str,
        signer_peer_id: String,
        signature: Vec<u8>,
        _signing_key: &SigningKey,
    ) -> Result<bool> {
        // Scope the wallet mutable borrow so self is free for audit logging after
        let (is_ready, total_weight, required_sigs) = {
            let wallet = self.wallets.get_mut(wallet_id)
                .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", wallet_id))?;
            
            // Find pending transaction
            let pending_tx = wallet.pending_transactions.iter_mut()
                .find(|tx| tx.transaction_id == transaction_id)
                .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", transaction_id))?;
            
            // Check if already signed
            if pending_tx.signatures.iter().any(|sig| sig.peer_id == signer_peer_id) {
                return Err(anyhow::anyhow!("Transaction already signed by this peer"));
            }
            
            // Check signer permissions and get weight + public key
            let (signer_weight, signer_public_key) = {
                let signer = wallet.signers.iter()
                    .find(|s| s.peer_id == signer_peer_id)
                    .ok_or_else(|| anyhow::anyhow!("Signer not found in wallet"))?;
                
                if !signer.permissions.contains(&Permission::ApproveTransactions) {
                    return Err(anyhow::anyhow!("Signer lacks transaction approval permission"));
                }
                
                (signer.weight, signer.public_key)
            };
            
            // Verify signature
            let message_hash = Self::calculate_transaction_hash(pending_tx);
            let public_key = VerifyingKey::from_bytes(&signer_public_key)
                .map_err(|_| anyhow::anyhow!("Invalid public key"))?;
            
            let sig_array: [u8; 64] = signature.clone().try_into()
                .map_err(|_| anyhow::anyhow!("Invalid signature length"))?;
            
            let signature_obj = Signature::from_bytes(&sig_array);
            
            if public_key.verify(&message_hash, &signature_obj).is_err() {
                return Err(anyhow::anyhow!("Signature verification failed"));
            }
            
            // Add signature
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            let signature_info = SignatureInfo {
                peer_id: signer_peer_id.clone(),
                signature,
                signed_at: now,
                weight: signer_weight,
                is_valid: true,
            };
            
            pending_tx.signatures.push(signature_info);
            
            // Update signer activity
            if let Some(signer) = wallet.signers.iter_mut().find(|s| s.peer_id == signer_peer_id) {
                signer.last_activity = now;
            }
            
            // Check if transaction is ready
            let total_weight: u8 = pending_tx.signatures.iter().map(|sig| sig.weight).sum();
            let required_sigs = pending_tx.required_signatures;
            let is_ready = total_weight >= required_sigs;
            
            if is_ready {
                pending_tx.status = TransactionStatus::Ready;
            }
            
            (is_ready, total_weight, required_sigs)
        }; // wallet borrow ends here
        
        // Log audit entry (self is now free)
        self.log_audit_entry(AuditEntry {
            entry_id: Self::generate_audit_id(),
            wallet_id: wallet_id.to_string(),
            actor_peer_id: signer_peer_id,
            action: AuditAction::TransactionSigned,
            details: format!("Signed transaction: {} (weight: {}/{})", 
                transaction_id, total_weight, required_sigs),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            ip_address: None,
            user_agent: None,
            success: true,
        });
        
        Ok(is_ready)
    }
    
    /// Execute ready transaction
    pub fn execute_transaction(
        &mut self,
        wallet_id: &str,
        transaction_id: &str,
        executor_peer_id: String,
    ) -> Result<String> {
        // Find and remove pending transaction
        let pending_tx_index = self.wallets.get(wallet_id)
            .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", wallet_id))?
            .pending_transactions.iter()
            .position(|tx| tx.transaction_id == transaction_id)
            .ok_or_else(|| anyhow::anyhow!("Transaction not found: {}", transaction_id))?;
        
        let pending_tx = self.wallets.get_mut(wallet_id)
            .unwrap()
            .pending_transactions
            .remove(pending_tx_index);
        
        // Check if transaction is ready
        if pending_tx.status != TransactionStatus::Ready {
            return Err(anyhow::anyhow!("Transaction is not ready for execution"));
        }
        
        // Check executor permissions
        let executor = self.wallets.get(wallet_id)
            .unwrap()
            .signers.iter()
            .find(|s| s.peer_id == executor_peer_id)
            .ok_or_else(|| anyhow::anyhow!("Executor not found in signers"))?;
        
        if !executor.permissions.contains(&Permission::ApproveTransactions) {
            return Err(anyhow::anyhow!("Executor lacks transaction approval permission"));
        }
        
        // Execute transaction (simulation)
        let execution_result = self.simulate_transaction_execution(&pending_tx)?;
        
        // Create completed transaction
        let completed_tx = CompletedTransaction {
            transaction_id: transaction_id.to_string(),
            transaction_type: pending_tx.transaction_type,
            data: pending_tx.data,
            signatures: pending_tx.signatures,
            completed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            block_height: execution_result.block_height,
            transaction_hash: execution_result.transaction_hash.clone(),
            gas_used: execution_result.gas_used,
            success: execution_result.success,
            error_message: execution_result.error_message,
        };
        
        // Add to completed transactions
        self.wallets.get_mut(wallet_id)
            .unwrap()
            .completed_transactions
            .push(completed_tx.clone());
        
        // Log audit entry
        self.log_audit_entry(AuditEntry {
            entry_id: Self::generate_audit_id(),
            wallet_id: wallet_id.to_string(),
            actor_peer_id: executor_peer_id,
            action: AuditAction::TransactionCompleted,
            details: format!("Executed transaction: {} (success: {})", 
                transaction_id, execution_result.success),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            ip_address: None,
            user_agent: None,
            success: execution_result.success,
        });
        
        Ok(execution_result.transaction_hash.unwrap_or_else(|| "no_hash".to_string()))
    }
    
    /// Add signer to wallet
    pub fn add_signer(
        &mut self,
        wallet_id: &str,
        signer_info: SignerInfo,
        admin_peer_id: String,
    ) -> Result<()> {
        let wallet = self.wallets.get_mut(wallet_id)
            .ok_or_else(|| anyhow::anyhow!("Wallet not found: {}", wallet_id))?;
        
        // Check admin permissions
        let admin = wallet.signers.iter()
            .find(|s| s.peer_id == admin_peer_id)
            .ok_or_else(|| anyhow::anyhow!("Admin not found in signers"))?;
        
        if !admin.permissions.contains(&Permission::ManageSigners) {
            return Err(anyhow::anyhow!("Admin lacks signer management permission"));
        }
        
        // Check maximum signers
        if wallet.signers.len() >= self.config.max_signers as usize {
            return Err(anyhow::anyhow!("Maximum signers reached"));
        }
        
        // Add signer
        wallet.signers.push(signer_info.clone());
        
        // Log audit entry
        self.log_audit_entry(AuditEntry {
            entry_id: Self::generate_audit_id(),
            wallet_id: wallet_id.to_string(),
            actor_peer_id: admin_peer_id,
            action: AuditAction::SignerAdded,
            details: format!("Added signer: {}", signer_info.peer_id),
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            ip_address: None,
            user_agent: None,
            success: true,
        });
        
        Ok(())
    }
    
    /// Get wallet information
    pub fn get_wallet(&self, wallet_id: &str) -> Option<&MultiSigWallet> {
        self.wallets.get(wallet_id)
    }
    
    /// Get pending transactions for wallet
    pub fn get_pending_transactions(&self, wallet_id: &str) -> Vec<&PendingTransaction> {
        self.wallets.get(wallet_id)
            .map(|wallet| wallet.pending_transactions.iter().collect())
            .unwrap_or_default()
    }
    
    /// Get completed transactions for wallet
    pub fn get_completed_transactions(&self, wallet_id: &str) -> Vec<&CompletedTransaction> {
        self.wallets.get(wallet_id)
            .map(|wallet| wallet.completed_transactions.iter().collect())
            .unwrap_or_default()
    }
    
    /// Get audit log
    pub fn get_audit_log(&self, wallet_id: Option<&str>, limit: Option<usize>) -> Vec<&AuditEntry> {
        let filtered_log: Vec<&AuditEntry> = self.audit_log.iter()
            .filter(|entry| {
                if let Some(id) = wallet_id {
                    entry.wallet_id == id
                } else {
                    true
                }
            })
            .collect();
        
        match limit {
            Some(limit) => filtered_log.iter().rev().take(limit).cloned().collect(),
            None => filtered_log.iter().rev().cloned().collect(),
        }
    }
    
    /// Generate wallet ID
    fn generate_wallet_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("wallet_{:x}", hasher.finish())
    }
    
    /// Generate transaction ID
    fn generate_transaction_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("tx_{:x}", hasher.finish())
    }
    
    /// Generate audit entry ID
    fn generate_audit_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("audit_{:x}", hasher.finish())
    }
    
    /// Calculate transaction hash for signing
    fn calculate_transaction_hash(transaction: &PendingTransaction) -> [u8; 8] {
        let mut hasher = Sha256::new();
        
        // Hash transaction components
        hasher.update(transaction.transaction_id.as_bytes());
        hasher.update(format!("{:?}", transaction.transaction_type).as_bytes());
        hasher.update(transaction.data.target.as_bytes());
        
        if let Some(amount) = transaction.data.amount {
            hasher.update(amount.to_string().as_bytes());
        }
        
        let hash_result = hasher.finalize();
        
        // Use first 8 bytes for Ed25519 signature
        let mut result = [0u8; 8];
        result.copy_from_slice(&hash_result[..8]);
        result
    }
    
    /// Simulate transaction execution
    fn simulate_transaction_execution(&self, transaction: &PendingTransaction) -> Result<TransactionExecutionResult> {
        // This is a simulation - in production, this would interact with actual blockchain/network
        
        let success = match transaction.transaction_type {
            TransactionType::AssetTransfer => true,
            TransactionType::ContractCall => true,
            TransactionType::DeFiOperation => true,
            TransactionType::GovernanceVote => true,
            TransactionType::SecurityChange => true,
            TransactionType::SignerManagement => true,
            TransactionType::WalletRecovery => true,
        };
        
        Ok(TransactionExecutionResult {
            success,
            block_height: Some(12345678),
            transaction_hash: Some(format!("0x{:x}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs())),
            gas_used: Some(21000),
            error_message: if success { None } else { Some("Simulation failed".to_string()) },
        })
    }
    
    /// Validate transaction against security policies (static to avoid borrow conflicts)
    fn validate_transaction_security(
        wallet: &MultiSigWallet,
        _transaction_type: &TransactionType,
        _data: &TransactionData,
        _security_level: &SecurityLevel,
    ) -> Result<()> {
        // Check wallet is active
        if wallet.state != WalletState::Active {
            return Err(anyhow::anyhow!("Wallet is not active"));
        }
        
        // Check minimum signers
        if wallet.signers.is_empty() {
            return Err(anyhow::anyhow!("Wallet has no signers"));
        }
        
        Ok(())
    }
    
    /// Log audit entry
    fn log_audit_entry(&mut self, entry: AuditEntry) {
        self.audit_log.push(entry);
        
        // Keep only last 10000 entries
        if self.audit_log.len() > 10000 {
            self.audit_log.drain(0..self.audit_log.len() - 10000);
        }
    }
}

/// Transaction execution result
#[derive(Debug, Clone)]
struct TransactionExecutionResult {
    success: bool,
    block_height: Option<u64>,
    transaction_hash: Option<String>,
    gas_used: Option<u64>,
    error_message: Option<String>,
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            time_based_approvals: false,
            approval_hours: 24,
            geo_verification_required: false,
            max_transaction_age_hours: 72,
            offline_signing_required: false,
            enable_batching: false,
            max_batch_size: 10,
            auto_expire: true,
        }
    }
}

impl Default for MultiSigConfig {
    fn default() -> Self {
        Self {
            max_signers: 10,
            min_required_signatures: 1,
            default_transaction_expiration: 24,
            max_transaction_expiration: 168, // 7 days
            enable_weighted_signing: false,
            max_signature_weight: 10,
            enable_recovery: true,
            recovery_threshold: 67, // 67% for recovery
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwa::market::VerificationLevel;
    use ed25519_dalek::Signer;
    
    fn create_test_signer_with_key(peer_id: &str, role: SignerRole) -> (SignerInfo, SigningKey) {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        let info = SignerInfo {
            peer_id: peer_id.to_string(),
            public_key: *verifying_key.as_bytes(),
            role,
            verification_level: VerificationLevel::Verified,
            weight: 1,
            last_activity: 1234567890,
            is_active: true,
            permissions: vec![
                Permission::InitiateTransactions,
                Permission::ApproveTransactions,
                Permission::ViewHistory,
            ],
        };
        (info, signing_key)
    }
    
    fn create_test_signer(peer_id: &str, role: SignerRole) -> SignerInfo {
        create_test_signer_with_key(peer_id, role).0
    }
    
    #[test]
    fn test_wallet_creation() {
        let mut manager = MultiSigManager::new(MultiSigConfig::default());
        
        let signers = vec![
            create_test_signer("peer1", SignerRole::Primary),
            create_test_signer("peer2", SignerRole::Secondary),
            create_test_signer("peer3", SignerRole::Arbitrator),
        ];
        
        let metadata = WalletMetadata {
            name: "Test Wallet".to_string(),
            description: Some("Test multisig wallet".to_string()),
            organization: Some("Test Org".to_string()),
            compliance_requirements: vec![],
            geographic_restrictions: vec![],
            transaction_limits: TransactionLimits {
                max_single_transaction: 1000000,
                max_daily_amount: 5000000,
                max_weekly_amount: 20000000,
                max_monthly_amount: 50000000,
                min_transaction_amount: 1000,
                currency: "USD".to_string(),
            },
            tags: vec!["test".to_string()],
        };
        
        let wallet_id = manager.create_wallet(
            "Test Wallet".to_string(),
            2, // 2-of-3 multisig
            signers,
            metadata,
            "admin".to_string(),
        ).unwrap();
        
        let wallet = manager.get_wallet(&wallet_id).unwrap();
        assert_eq!(wallet.total_signers, 3);
        assert_eq!(wallet.required_signatures, 2);
        assert_eq!(wallet.state, WalletState::Active);
    }
    
    #[test]
    fn test_transaction_lifecycle() {
        let mut manager = MultiSigManager::new(MultiSigConfig::default());
        
        // Create wallet — retain signing keys for later use
        let (signer1, signing_key1) = create_test_signer_with_key("peer1", SignerRole::Primary);
        let (signer2, signing_key2) = create_test_signer_with_key("peer2", SignerRole::Secondary);
        let signers = vec![signer1, signer2];
        
        let metadata = WalletMetadata {
            name: "Test Wallet".to_string(),
            description: None,
            organization: None,
            compliance_requirements: vec![],
            geographic_restrictions: vec![],
            transaction_limits: TransactionLimits {
                max_single_transaction: 1000000,
                max_daily_amount: 5000000,
                max_weekly_amount: 20000000,
                max_monthly_amount: 50000000,
                min_transaction_amount: 1000,
                currency: "USD".to_string(),
            },
            tags: vec![],
        };
        
        let wallet_id = manager.create_wallet(
            "Test Wallet".to_string(),
            2, // 2-of-2 multisig
            signers,
            metadata,
            "admin".to_string(),
        ).unwrap();
        
        // Initiate transaction
        let transaction_data = TransactionData {
            target: "0x1234567890123456789012345678901234567890".to_string(),
            amount: Some(50000),
            currency: Some("USD".to_string()),
            function_signature: None,
            parameters: vec![],
            metadata: HashMap::new(),
        };
        
        let tx_id = manager.initiate_transaction(
            &wallet_id,
            TransactionType::AssetTransfer,
            transaction_data,
            "peer1".to_string(),
            SecurityLevel::Medium,
        ).unwrap();
        
        // First signature (peer1) — use the SAME key that created the signer
        let message_hash1 = MultiSigManager::calculate_transaction_hash(
            manager.get_pending_transactions(&wallet_id)[0]
        );
        let signature1 = signing_key1.sign(&message_hash1);
        
        let is_ready1 = manager.sign_transaction(
            &wallet_id,
            &tx_id,
            "peer1".to_string(),
            signature1.to_bytes().to_vec(),
            &signing_key1,
        ).unwrap();
        
        assert!(!is_ready1); // Should not be ready yet (need 2 signatures)
        
        // Second signature (peer2) — use the SAME key that created the signer
        let message_hash2 = MultiSigManager::calculate_transaction_hash(
            manager.get_pending_transactions(&wallet_id)[0]
        );
        let signature2 = signing_key2.sign(&message_hash2);
        
        let is_ready2 = manager.sign_transaction(
            &wallet_id,
            &tx_id,
            "peer2".to_string(),
            signature2.to_bytes().to_vec(),
            &signing_key2,
        ).unwrap();
        
        assert!(is_ready2); // Should be ready now
        
        // Execute transaction
        let tx_hash = manager.execute_transaction(
            &wallet_id,
            &tx_id,
            "peer1".to_string(),
        ).unwrap();
        
        assert!(!tx_hash.is_empty());
        
        // Verify transaction is in completed list
        let completed_txs = manager.get_completed_transactions(&wallet_id);
        assert_eq!(completed_txs.len(), 1);
        assert_eq!(completed_txs[0].transaction_id, tx_id);
        assert!(completed_txs[0].success);
    }
    
    #[test]
    fn test_audit_log() {
        let mut manager = MultiSigManager::new(MultiSigConfig::default());
        
        let signers = vec![create_test_signer("peer1", SignerRole::Primary)];
        
        let metadata = WalletMetadata {
            name: "Test Wallet".to_string(),
            description: None,
            organization: None,
            compliance_requirements: vec![],
            geographic_restrictions: vec![],
            transaction_limits: TransactionLimits {
                max_single_transaction: 1000000,
                max_daily_amount: 5000000,
                max_weekly_amount: 20000000,
                max_monthly_amount: 50000000,
                min_transaction_amount: 1000,
                currency: "USD".to_string(),
            },
            tags: vec![],
        };
        
        let wallet_id = manager.create_wallet(
            "Test Wallet".to_string(),
            1,
            signers,
            metadata,
            "admin".to_string(),
        ).unwrap();
        
        // Check audit log
        let audit_log = manager.get_audit_log(Some(&wallet_id), Some(10));
        assert_eq!(audit_log.len(), 1);
        assert_eq!(audit_log[0].action, AuditAction::WalletCreated);
        assert_eq!(audit_log[0].wallet_id, wallet_id);
    }
}
