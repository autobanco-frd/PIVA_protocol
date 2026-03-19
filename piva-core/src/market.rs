//! Marketplace Module for PIVA Protocol
//! 
//! Implements cryptographic market offers with peer scoring and ISO 20022 compatibility.
//! This enables PIVA assets to be traded in a trust-minimized global marketplace.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use piva_crypto::hash_sha3_256;
use crate::asset::AssetId;

/// Market Offer Structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketOffer {
    /// Unique offer identifier
    pub offer_id: String,
    
    /// Asset being offered
    pub asset_id: AssetId,
    
    /// Offer price in smallest currency unit (e.g., satoshis, cents)
    pub price: u64,
    
    /// Currency code (ISO 4217)
    pub currency: String,
    
    /// Offer expiration timestamp (Unix timestamp)
    pub expiration: u64,
    
    /// Offer creator's node ID
    pub creator_node_id: String,
    
    /// Creator's peer score
    pub peer_score: PeerScore,
    
    /// Offer type (buy/sell)
    pub offer_type: OfferType,
    
    /// Minimum amount for trade
    pub min_amount: u64,
    
    /// Maximum amount available
    pub max_amount: u64,
    
    /// Geographic location (optional, for local trades)
    pub location: Option<GeoLocation>,
    
    /// Payment methods accepted
    pub payment_methods: Vec<PaymentMethod>,
    
    /// Offer metadata
    pub metadata: OfferMetadata,
    
    /// Creation timestamp
    pub created_at: u64,
    
    /// Network mode
    pub network: String,
}

/// Offer Type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OfferType {
    /// Buy order (want to acquire asset)
    Buy,
    /// Sell order (want to sell asset)
    Sell,
}

/// Peer Score for Reputation System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerScore {
    /// Overall score (0-1000)
    pub overall_score: u16,
    
    /// Number of successful trades
    pub successful_trades: u32,
    
    /// Number of failed trades
    pub failed_trades: u32,
    
    /// Total volume traded (in base currency)
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

/// Verification Level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationLevel {
    /// Unverified (new account)
    Unverified,
    /// Basic (identity verified)
    Basic,
    /// Trusted (successful trades > 10)
    Trusted,
    /// Premium (high volume + long history)
    Premium,
}

/// Trust Factor Components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustFactor {
    /// Factor type
    pub factor_type: TrustFactorType,
    
    /// Factor score (0-100)
    pub score: u8,
    
    /// Evidence or proof
    pub evidence: Option<String>,
    
    /// Timestamp of last verification
    pub last_verified: u64,
}

/// Trust Factor Types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustFactorType {
    /// Identity verification
    Identity,
    /// Successful HTLC completions
    HtlcSuccess,
    /// Long-term node operation
    NodeUptime,
    /// Geographic proximity
    LocalPresence,
    /// Financial history
    FinancialHistory,
}

/// Geographic Location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// Country code (ISO 3166-1 alpha-2)
    pub country: String,
    
    /// Region/state
    pub region: Option<String>,
    
    /// City
    pub city: Option<String>,
    
    /// Latitude
    pub latitude: Option<f64>,
    
    /// Longitude
    pub longitude: Option<f64>,
    
    /// Radius in kilometers for local trades
    pub radius_km: Option<u32>,
}

/// Payment Methods
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PaymentMethod {
    /// Lightning Network
    Lightning,
    /// Bank transfer (ISO 20022 compatible)
    BankTransfer,
    /// Cash in person
    Cash,
    /// Cryptocurrency
    Crypto(String), // Symbol like "BTC", "ETH"
    /// Mobile payment
    Mobile(String), // Provider like "Venmo", "Zelle"
}

/// Offer Metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfferMetadata {
    /// Offer description
    pub description: Option<String>,
    
    /// Terms and conditions
    pub terms: Option<String>,
    
    /// Contact information
    pub contact: Option<ContactInfo>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Language codes (ISO 639-1)
    pub languages: Vec<String>,
}

/// Contact Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactInfo {
    /// Preferred contact method
    pub preferred_method: PaymentMethod,
    
    /// Contact identifier
    pub identifier: String,
    
    /// Availability hours
    pub availability: Option<String>,
}

/// Marketplace Manager
pub struct MarketplaceManager {
    network: String,
}

impl MarketplaceManager {
    /// Create new marketplace manager
    pub fn new(network: &str) -> Self {
        Self {
            network: network.to_string(),
        }
    }
    
    /// Create a new market offer
    pub fn create_offer(
        &self,
        asset_id: AssetId,
        price: u64,
        currency: String,
        expiration_hours: u64,
        creator_node_id: String,
        peer_score: PeerScore,
        offer_type: OfferType,
        min_amount: u64,
        max_amount: u64,
        payment_methods: Vec<PaymentMethod>,
        metadata: OfferMetadata,
    ) -> Result<MarketOffer> {
        // Generate offer ID
        let offer_id = self.generate_offer_id(&asset_id, &creator_node_id);
        
        // Calculate expiration timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiration = now + (expiration_hours * 3600);
        
        let offer = MarketOffer {
            offer_id,
            asset_id,
            price,
            currency,
            expiration,
            creator_node_id,
            peer_score,
            offer_type,
            min_amount,
            max_amount,
            location: None, // Can be set separately
            payment_methods,
            metadata,
            created_at: now,
            network: self.network.clone(),
        };
        
        Ok(offer)
    }
    
    /// Validate offer integrity
    pub fn validate_offer(&self, offer: &MarketOffer) -> Result<bool> {
        // Check expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if offer.expiration <= now {
            return Ok(false);
        }
        
        // Verify offer ID
        let expected_id = self.generate_offer_id(&offer.asset_id, &offer.creator_node_id);
        if expected_id != offer.offer_id {
            return Ok(false);
        }
        
        // Validate price range
        if offer.price == 0 || offer.min_amount == 0 || offer.max_amount < offer.min_amount {
            return Ok(false);
        }
        
        // Validate peer score
        if offer.peer_score.overall_score > 1000 {
            return Ok(false);
        }
        
        Ok(true)
    }
    
    /// Calculate peer score from trading history
    pub fn calculate_peer_score(
        &self,
        successful_trades: u32,
        failed_trades: u32,
        total_volume: u64,
        account_age_days: u32,
        last_activity: u64,
        verification_level: VerificationLevel,
        trust_factors: Vec<TrustFactor>,
    ) -> PeerScore {
        // Base score from success rate
        let total_trades = successful_trades + failed_trades;
        let success_rate = if total_trades > 0 {
            (successful_trades * 1000) / total_trades
        } else {
            500 // Neutral score for new accounts
        };
        
        // Volume bonus (up to 200 points)
        let volume_bonus = std::cmp::min(total_volume / 1000000, 200);
        
        // Age bonus (up to 100 points)
        let age_bonus = std::cmp::min(account_age_days / 10, 100);
        
        // Verification bonus
        let verification_bonus = match verification_level {
            VerificationLevel::Unverified => 0,
            VerificationLevel::Basic => 50,
            VerificationLevel::Trusted => 150,
            VerificationLevel::Premium => 300,
        };
        
        // Trust factors bonus
        let trust_bonus: u16 = trust_factors.iter().map(|tf| tf.score as u16).sum();
        
        let overall_score = std::cmp::min(
            success_rate + volume_bonus + age_bonus + verification_bonus + trust_bonus,
            1000
        );
        
        PeerScore {
            overall_score,
            successful_trades,
            failed_trades,
            total_volume,
            account_age_days,
            last_activity,
            verification_level,
            trust_factors,
        }
    }
    
    /// Generate offer ID
    fn generate_offer_id(&self, asset_id: &AssetId, creator_node_id: &str) -> String {
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(asset_id.to_string().as_bytes());
        hasher.update(creator_node_id.as_bytes());
        hasher.update(self.network.as_bytes());
        let hash = hasher.finalize();
        
        format!("offer_{}", hex::encode(hash))
    }
    
    /// Filter offers by criteria
    pub fn filter_offers(
        &self,
        offers: &[MarketOffer],
        asset_type: Option<String>,
        currency: Option<String>,
        min_peer_score: Option<u16>,
        location: Option<GeoLocation>,
        max_distance_km: Option<u32>,
    ) -> Vec<&MarketOffer> {
        offers.iter().filter(|offer| {
            // Filter by asset type
            if let Some(ref asset_filter) = asset_type {
                if !offer.asset_id.to_string().contains(asset_filter) {
                    return false;
                }
            }
            
            // Filter by currency
            if let Some(ref currency_filter) = currency {
                if offer.currency != *currency_filter {
                    return false;
                }
            }
            
            // Filter by peer score
            if let Some(min_score) = min_peer_score {
                if offer.peer_score.overall_score < min_score {
                    return false;
                }
            }
            
            // Filter by location
            if let (Some(ref user_location), Some(max_dist)) = (location, max_distance_km) {
                if let Some(ref offer_location) = offer.location {
                    if !self.is_within_distance(user_location, offer_location, max_dist) {
                        return false;
                    }
                } else {
                    return false; // Offer has no location specified
                }
            }
            
            true
        }).collect()
    }
    
    /// Check if two locations are within distance
    fn is_within_distance(
        &self,
        user_location: &GeoLocation,
        offer_location: &GeoLocation,
        max_distance_km: u32,
    ) -> bool {
        match (user_location.latitude, user_location.longitude, 
               offer_location.latitude, offer_location.longitude) {
            (Some(lat1), Some(lon1), Some(lat2), Some(lon2)) => {
                // Simple distance calculation (Haversine formula)
                let lat1_rad = lat1.to_radians();
                let lat2_rad = lat2.to_radians();
                let delta_lat = (lat2 - lat1).to_radians();
                let delta_lon = (lon2 - lon1).to_radians();
                
                let a = (delta_lat / 2.0).sin().powi(2) +
                        lat1_rad.cos() * lat2_rad.cos() *
                        (delta_lon / 2.0).sin().powi(2);
                let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
                
                let earth_radius_km = 6371.0;
                let distance = earth_radius_km * c;
                
                distance <= max_distance_km as f64
            },
            _ => false, // Missing coordinates
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_offer_creation() {
        let manager = MarketplaceManager::new("devnet");
        let asset_id = "test_asset_123".parse().unwrap();
        let peer_score = PeerScore {
            overall_score: 750,
            successful_trades: 25,
            failed_trades: 2,
            total_volume: 5000000,
            account_age_days: 180,
            last_activity: 1234567890,
            verification_level: VerificationLevel::Trusted,
            trust_factors: vec![],
        };
        
        let offer = manager.create_offer(
            asset_id,
            100000, // 1000 USD in cents
            "USD".to_string(),
            24, // 24 hours
            "node_abc123".to_string(),
            peer_score,
            OfferType::Sell,
            10000, // 100 USD min
            100000, // 1000 USD max
            vec![PaymentMethod::Lightning, PaymentMethod::BankTransfer],
            OfferMetadata {
                description: Some("Selling digital art certificate".to_string()),
                terms: Some("Payment within 2 hours".to_string()),
                contact: None,
                tags: vec!["digital".to_string(), "art".to_string()],
                languages: vec!["en".to_string(), "es".to_string()],
            },
        ).unwrap();
        
        assert_eq!(offer.offer_type, OfferType::Sell);
        assert_eq!(offer.currency, "USD");
        assert_eq!(offer.price, 100000);
        assert_eq!(offer.min_amount, 10000);
        assert_eq!(offer.max_amount, 100000);
        assert!(manager.validate_offer(&offer).unwrap());
    }
    
    #[test]
    fn test_peer_score_calculation() {
        let manager = MarketplaceManager::new("devnet");
        
        let peer_score = manager.calculate_peer_score(
            50,  // successful trades
            2,   // failed trades
            1000000, // total volume
            365, // account age days
            1234567890, // last activity
            VerificationLevel::Trusted,
            vec![
                TrustFactor {
                    factor_type: TrustFactorType::Identity,
                    score: 80,
                    evidence: Some("KYC verified".to_string()),
                    last_verified: 1234567890,
                },
                TrustFactor {
                    factor_type: TrustFactorType::HtlcSuccess,
                    score: 90,
                    evidence: None,
                    last_verified: 1234567890,
                },
            ],
        );
        
        assert!(peer_score.overall_score > 800);
        assert_eq!(peer_score.successful_trades, 50);
        assert_eq!(peer_score.failed_trades, 2);
        assert_eq!(peer_score.verification_level, VerificationLevel::Trusted);
    }
}
