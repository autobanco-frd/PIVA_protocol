//! # Proof of Productivity (PoP) Scoring System
//!
//! Métrica de confianza local basada en desempeño real para la antifragilidad de la red.

use serde::{Serialize, Deserialize};
use thiserror::Error;
use redb::Value;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Error, Debug)]
pub enum ScoringError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Invalid score value: {0}")]
    InvalidScore(String),
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
}

/// Achievement types for scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Achievement {
    /// +1 point: Successful BLAKE3 chunk delivery
    ChunkDelivery { 
        chunk_hash: [u8; 32], 
        delivery_time_ms: u64 
    },
    /// +5 points: Successful RWA notarization
    Notarization { 
        asset_id: [u8; 32], 
        verification_time_ms: u64 
    },
    /// -10 points: Hash inconsistency (fraud detection)
    FraudDetection { 
        asset_id: [u8; 32], 
        expected_hash: [u8; 32], 
        actual_hash: [u8; 32] 
    },
    /// -3 points: Failed transfer or corrupted data
    TransferFailure { 
        asset_id: [u8; 32], 
        reason: String 
    },
}

/// Peer reputation score with achievement tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerScore {
    pub peer_id: [u8; 32],
    pub base_score: i32,           // Starting score: 500
    pub achievement_score: i32,    // From achievements
    pub latency_score: f32,        // From response time
    pub decay_factor: f32,         // Time-based decay
    pub total_score: i32,          // Final calculated score
    
    // Achievement counters
    pub successful_deliveries: u64,
    pub successful_notarizations: u64,
    pub fraud_detections: u64,
    pub failed_transfers: u64,
    
    // Timing metrics
    pub last_seen: u64,
    pub average_latency_ms: f32,
    pub uptime_ratio: f32,
    
    // Achievement history (last N achievements)
    pub recent_achievements: Vec<Achievement>,
}

impl Default for PeerScore {
    fn default() -> Self {
        Self {
            peer_id: [0u8; 32],
            base_score: 500,
            achievement_score: 0,
            latency_score: 0.0,
            decay_factor: 1.0,
            total_score: 0,  // Comienza en 0, se calcula en calculate_score()
            
            successful_deliveries: 0,
            successful_notarizations: 0,
            fraud_detections: 0,
            failed_transfers: 0,
            
            last_seen: now_secs(),
            average_latency_ms: 0.0,
            uptime_ratio: 0.0,
            
            recent_achievements: Vec::new(),
        }
    }
}

impl PeerScore {
    /// Create a new peer score with default values
    pub fn new(peer_id: [u8; 32]) -> Self {
        let mut score = Self {
            peer_id,
            base_score: 500,
            achievement_score: 0,
            latency_score: 0.0,
            decay_factor: 1.0,
            total_score: 0,
            
            successful_deliveries: 0,
            successful_notarizations: 0,
            fraud_detections: 0,
            failed_transfers: 0,
            
            last_seen: now_secs(),
            average_latency_ms: 0.0,
            uptime_ratio: 0.0,
            
            recent_achievements: Vec::new(),
        };
        score.calculate_score();
        score
    }
    
    /// Calculate final score based on all factors
    pub fn calculate_score(&mut self) {
        // Achievement score calculation
        self.achievement_score = 
            (self.successful_deliveries as i32 * 1) +
            (self.successful_notarizations as i32 * 5) -
            (self.fraud_detections as i32 * 10) -
            (self.failed_transfers as i32 * 3);
        
        // Latency score (lower latency = higher score, but no bonus for perfect latency)
        // 0ms = 0, 1000ms = -50, >2000ms = more negative
        // En calculate_score()
        self.latency_score = if self.average_latency_ms <= 1000.0 {
            // 0ms = 0 puntos de penalización, 1000ms = -50 puntos
            -(self.average_latency_ms * 0.05) 
        } else {
            -50.0 - ((self.average_latency_ms - 1000.0) / 10.0) // Penalización más agresiva > 1s
        };
        
        // Apply time decay (reduce score over time without activity)
        let hours_inactive = (now_secs() - self.last_seen) as f32 / 3600.0;
        self.decay_factor = (-hours_inactive / 168.0).exp(); // Decay over 1 week
        
        // Final score calculation
        let pre_decay = self.base_score + self.achievement_score + self.latency_score as i32;
        self.total_score = (pre_decay as f32 * self.decay_factor) as i32;
        
        // Clamp score to reasonable bounds
        self.total_score = self.total_score.clamp(-1000, 2000);
    }
    
    /// Record an achievement and update scores
    pub fn record_achievement(&mut self, achievement: Achievement) {
        // Update counters based on achievement type
        match &achievement {
            Achievement::ChunkDelivery { .. } => {
                self.successful_deliveries += 1;
            }
            Achievement::Notarization { .. } => {
                self.successful_notarizations += 1;
            }
            Achievement::FraudDetection { .. } => {
                self.fraud_detections += 1;
            }
            Achievement::TransferFailure { .. } => {
                self.failed_transfers += 1;
            }
        }
        
        // Add to recent achievements (keep last 100)
        self.recent_achievements.push(achievement);
        if self.recent_achievements.len() > 100 {
            self.recent_achievements.remove(0);
        }
        
        self.last_seen = now_secs();
        self.calculate_score();
    }
    
    /// Update latency metrics
    pub fn update_latency(&mut self, latency_ms: u64) {
        // Exponential moving average for latency
        let alpha = 0.1; // Smoothing factor
        self.average_latency_ms = 
            (alpha * latency_ms as f32) + ((1.0 - alpha) * self.average_latency_ms);
        
        self.calculate_score();
    }
    
    /// Update uptime ratio (0.0 to 1.0)
    pub fn update_uptime(&mut self, uptime_ratio: f32) {
        self.uptime_ratio = uptime_ratio.clamp(0.0, 1.0);
        self.calculate_score();
    }
    
    /// Get trust level based on score
    pub fn trust_level(&self) -> TrustLevel {
        match self.total_score {
            score if score >= 500 => TrustLevel::Excellent,  // 501 será Excellent
            score if score >= 350 => TrustLevel::Good,      // 350-549
            score if score >= 200 => TrustLevel::Neutral,   // 200-349
            score if score >= 50  => TrustLevel::Poor,       // 50-199
            score if score >= 0   => TrustLevel::VeryPoor,   // 0-49
            _ => TrustLevel::Banned,                      // < 0
        }
    }
    
    /// Check if peer is trustworthy for operations
    pub fn is_trustworthy(&self) -> bool {
        self.total_score >= 300 && self.fraud_detections == 0
    }
}

/// Trust levels for peer classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TrustLevel {
    Excellent,  // 800+
    Good,        // 600-799
    Neutral,     // 400-599
    Poor,        // 200-399
    VeryPoor,    // 0-199
    Banned,      // < 0
}

/// Achievement aggregator to prevent database saturation
#[derive(Debug, Clone)]
pub struct AchievementAggregator {
    pending_achievements: Vec<( [u8; 32], Achievement)>,
    last_flush: u64,
    flush_interval_ms: u64,
}

impl AchievementAggregator {
    pub fn new() -> Self {
        Self {
            pending_achievements: Vec::new(),
            last_flush: now_secs(),
            flush_interval_ms: 300_000, // 5 minutes
        }
    }
    
    /// Add achievement to pending queue
    pub fn add_achievement(&mut self, peer_id: [u8; 32], achievement: Achievement) {
        self.pending_achievements.push((peer_id, achievement));
        
        // Check if we should flush (100 achievements or time interval)
        if self.pending_achievements.len() >= 100 || 
           (now_secs() - self.last_flush) * 1000 >= self.flush_interval_ms {
            self.flush();
        }
    }
    
    /// Get pending achievements and clear queue
    pub fn flush(&mut self) -> Vec<( [u8; 32], Achievement)> {
        let achievements = std::mem::take(&mut self.pending_achievements);
        self.last_flush = now_secs();
        achievements
    }
    
    /// Check if there are pending achievements
    pub fn has_pending(&self) -> bool {
        !self.pending_achievements.is_empty()
    }
}

impl Default for AchievementAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement redb::Value for PeerScore
impl Value for PeerScore {
    type SelfType<'a> = PeerScore;
    type AsBytes<'a> = Vec<u8>;
    
    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).expect("Failed to deserialize PeerScore")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        bincode::serialize(value).expect("Failed to serialize PeerScore")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("PeerScore")
    }
}

/// Implement redb::Value for Achievement
impl Value for Achievement {
    type SelfType<'a> = Achievement;
    type AsBytes<'a> = Vec<u8>;
    
    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        bincode::deserialize(data).expect("Failed to deserialize Achievement")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        bincode::serialize(value).expect("Failed to serialize Achievement")
    }

    fn type_name() -> redb::TypeName {
        redb::TypeName::new("Achievement")
    }
}

/// Get current timestamp in seconds
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_peer_score_creation() {
        let peer_id = [1u8; 32];
        let score = PeerScore::new(peer_id);
        
        assert_eq!(score.total_score, 500);
        assert_eq!(score.base_score, 500);
        assert_eq!(score.achievement_score, 0);
    }
    
    #[test]
    fn test_chunk_delivery_scoring() {
        let mut score = PeerScore::new([1u8; 32]);
        
        let achievement = Achievement::ChunkDelivery {
            chunk_hash: [2u8; 32],
            delivery_time_ms: 100,
        };
        
        score.record_achievement(achievement);
        
        assert_eq!(score.successful_deliveries, 1);
        assert_eq!(score.total_score, 501); // 500 + 1
    }
    
    #[test]
    fn test_notarization_scoring() {
        let mut score = PeerScore::new([1u8; 32]);
        
        let achievement = Achievement::Notarization {
            asset_id: [2u8; 32],
            verification_time_ms: 200,
        };
        
        score.record_achievement(achievement);
        
        assert_eq!(score.successful_notarizations, 1);
        assert_eq!(score.total_score, 505); // 500 + 5
    }
    
    #[test]
    fn test_fraud_detection_scoring() {
        let mut score = PeerScore::new([1u8; 32]);
        
        let achievement = Achievement::FraudDetection {
            asset_id: [2u8; 32],
            expected_hash: [3u8; 32],
            actual_hash: [4u8; 32],
        };
        
        score.record_achievement(achievement);
        
        assert_eq!(score.fraud_detections, 1);
        assert_eq!(score.total_score, 490); // 500 - 10
    }
    
    #[test]
    fn test_latency_scoring() {
        let mut score = PeerScore::new([1u8; 32]);
        
        // Good latency (100ms) - debería tener penalización mínima
        score.update_latency(100);
        assert!(score.latency_score > -10.0); // -5.0 con nueva fórmula, debe ser > -10.0
        
        // Poor latency (1500ms)
        score.update_latency(1500);
        assert!(score.latency_score < 0.0);
    }
    
    #[test]
    fn test_trust_levels() {
        let mut score = PeerScore::new([1u8; 32]);
        
        // Excellent
        score.total_score = 900;
        assert!(matches!(score.trust_level(), TrustLevel::Excellent));
        assert!(score.is_trustworthy());
        
        // Banned
        score.total_score = -100;
        assert!(matches!(score.trust_level(), TrustLevel::Banned));
        assert!(!score.is_trustworthy());
    }
    
    #[test]
    fn test_achievement_aggregator() {
        let mut aggregator = AchievementAggregator::new();
        
        let peer_id = [1u8; 32];
        let achievement = Achievement::ChunkDelivery {
            chunk_hash: [2u8; 32],
            delivery_time_ms: 100,
        };
        
        aggregator.add_achievement(peer_id, achievement);
        assert!(aggregator.has_pending());
        
        let pending = aggregator.flush();
        assert_eq!(pending.len(), 1);
        assert!(!aggregator.has_pending());
    }
}
