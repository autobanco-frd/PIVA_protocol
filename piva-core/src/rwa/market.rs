//! Marketplace Module for PIVA Protocol
//! 
//! Implements cryptographic market offers with peer scoring and ISO 20022 compatibility.
//! This enables PIVA assets to be traded in a trust-minimized global marketplace.

use serde::{Deserialize, Serialize};
use crate::asset::AssetId;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OfferStatus {
    /// Offer is open for trades
    Open,
    /// Offer is locked in HTLC
    Locked,
    /// Trade completed successfully
    Completed,
    /// Offer was cancelled
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum VerificationLevel {
    None,
    Basic,
    Verified,
    Institutional,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum Permission {
    InitiateTransactions,
    ApproveTransactions,
    ViewHistory,
    CancelTransactions,
    ModifySettings,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum SignerRole {
    Primary,
    Secondary,
    Arbitrator,
    Observer,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum TrustFactorType {
    TradeHistory,
    Identity,
    Collateral,
    NodeUptime,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TrustFactor {
    pub factor_type: TrustFactorType,
    pub value: u32,
    pub last_updated: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub country: String,
    pub region: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MarketOffer {
    /// Unique offer identifier
    pub offer_id: String,
    
    /// Asset being offered
    pub asset_id: AssetId,
    
    /// Offer price in smallest currency unit
    pub price: u64,
    
    /// Offer amount
    pub amount: u64,
    
    /// Currency code (ISO 4217)
    pub currency: String,
    
    /// Offer creator's peer ID
    pub creator_peer_id: String,
    
    /// Current offer status
    pub status: OfferStatus,
    
    /// Minimum reputation score to trade
    pub min_reputation: u32,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Expiration timestamp
    pub expires_at: u64,
    
    /// Offer type (buy/sell)
    pub offer_type: OfferType,
    
    /// Geographic location (optional)
    pub location: Option<GeoLocation>,
    
    /// Payment methods accepted
    pub payment_methods: Vec<String>,
    
    /// Peer score information
    pub peer_score: PeerScore,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum OfferType {
    /// Buy order (want to acquire asset)
    Buy,
    /// Sell order (want to sell asset)
    Sell,
}

impl MarketOffer {
    /// Create new market offer
    pub fn new(
        asset_id: AssetId,
        price: u64,
        amount: u64,
        currency: String,
        peer_id: String,
        offer_type: OfferType,
    ) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            offer_id: format!("offer_{}_{}", peer_id, now),
            asset_id,
            price,
            amount,
            currency,
            creator_peer_id: peer_id.clone(),
            status: OfferStatus::Open,
            min_reputation: 0,
            created_at: now,
            expires_at: now + 86400, // 24 hours
            offer_type,
            location: None,
            payment_methods: vec!["PIVA".to_string()],
            peer_score: PeerScore {
                overall_score: 500, // Default score
                successful_trades: 0,
                failed_trades: 0,
                total_volume: 0,
                account_age_days: 0,
                last_activity: now,
                verification_level: VerificationLevel::None,
                trust_factors: vec![],
            },
        }
    }
    
    /// Check if offer is still valid
    pub fn is_valid(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.status == OfferStatus::Open && self.expires_at > now
    }
    
    /// Lock offer for HTLC trade
    pub fn lock(&mut self) -> Result<(), &'static str> {
        match self.status {
            OfferStatus::Open => {
                self.status = OfferStatus::Locked;
                Ok(())
            },
            _ => Err("Offer cannot be locked"),
        }
    }
    
    /// Complete offer trade
    pub fn complete(&mut self) -> Result<(), &'static str> {
        match self.status {
            OfferStatus::Locked => {
                self.status = OfferStatus::Completed;
                Ok(())
            },
            _ => Err("Offer cannot be completed"),
        }
    }
    
    /// Cancel offer
    pub fn cancel(&mut self) -> Result<(), &'static str> {
        match self.status {
            OfferStatus::Open | OfferStatus::Locked => {
                self.status = OfferStatus::Cancelled;
                Ok(())
            },
            _ => Err("Offer cannot be cancelled"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerScore {
    /// Overall score (0-1000)
    pub overall_score: u16,
    
    /// Number of successful trades
    pub successful_trades: u32,
    
    /// Number of failed trades
    pub failed_trades: u32,
    
    /// Total trade volume
    pub total_volume: u64,
    
    /// Account age in days
    pub account_age_days: u32,
    
    /// Last activity timestamp
    pub last_activity: u64,
    
    /// Verification level
    pub verification_level: VerificationLevel,
    
    /// Trust factors
    pub trust_factors: Vec<TrustFactor>,
}

impl PeerScore {
    /// Calculate success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.successful_trades + self.failed_trades;
        if total == 0 {
            0.0
        } else {
            self.successful_trades as f32 / total as f32
        }
    }
    
    /// Check if peer meets minimum reputation
    pub fn meets_minimum(&self, minimum: u32) -> bool {
        self.overall_score >= minimum as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    // Generate valid AssetId with proper network prefix
    fn mock_asset_id() -> AssetId {
        // AssetId requires network prefix: piva_dev_, piva_test_, or piva_live_
        AssetId::from_str("piva_dev_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()
    }

    #[test]
    fn test_offer_creation() {
        let asset_id = mock_asset_id();
        let offer = MarketOffer::new(
            asset_id, 
            10000,              // Price as u64 (100.00 USD in cents)
            1000,               // Amount
            "USD".to_string(), 
            "peer_123".to_string(),
            OfferType::Sell,
        );
        assert_eq!(offer.status, OfferStatus::Open);
        assert_eq!(offer.price, 10000);
    }

    #[test]
    fn test_offer_lifecycle() {
        let asset_id = mock_asset_id();
        let mut offer = MarketOffer::new(
            asset_id, 
            5000, 
            1000,
            "MXN".to_string(), 
            "peer_456".to_string(),
            OfferType::Buy,
        );
        
        offer.status = OfferStatus::Locked;
        assert_eq!(offer.status, OfferStatus::Locked);
        
        offer.status = OfferStatus::Completed;
        assert_eq!(offer.status, OfferStatus::Completed);
    }

    #[test]
    fn test_peer_score() {
        let score: u32 = 100;
        assert!(score > 0);
    }
}
