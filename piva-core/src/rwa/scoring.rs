//! Advanced Peer Scoring System with Trust Decay and Geographic Clustering
//! 
//! Implements sophisticated reputation algorithms with temporal decay,
//! geographic clustering, and trust factor analysis.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::time::{SystemTime, UNIX_EPOCH};
use anyhow::Result;
use crate::rwa::market::{TrustFactor, TrustFactorType, VerificationLevel, GeoLocation};

/// Advanced scoring engine with temporal dynamics
pub struct AdvancedScoringEngine {
    /// Peer data storage
    peers: HashMap<String, PeerData>,
    
    /// Geographic clustering data
    geo_clusters: HashMap<String, GeoCluster>,
    
    /// Scoring configuration
    config: ScoringConfig,
    
    /// Historical data for trend analysis
    #[allow(dead_code)]
    history: BTreeMap<u64, ScoreSnapshot>,
    
    /// Global statistics
    global_stats: GlobalScoringStats,
}

/// Complete peer data for advanced scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerData {
    /// Peer ID
    pub peer_id: String,
    
    /// Current score
    pub current_score: u16,
    
    /// Base score (without temporal decay)
    pub base_score: u16,
    
    /// Historical scores for trend analysis
    pub score_history: Vec<ScoreSnapshot>,
    
    /// Trust factors
    pub trust_factors: Vec<TrustFactor>,
    
    /// Trade history
    pub trade_history: Vec<TradeRecord>,
    
    /// Geographic locations
    pub locations: Vec<LocationRecord>,
    
    /// Activity patterns
    pub activity_patterns: ActivityPatterns,
    
    /// Reputation decay data
    pub decay_data: DecayData,
    
    /// Verification level
    pub verification_level: VerificationLevel,
    
    /// Last updated timestamp
    pub last_updated: u64,
}

/// Score snapshot at a specific time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreSnapshot {
    pub timestamp: u64,
    pub score: u16,
    pub reason: String,
    pub decay_factor: f32,
}

/// Trade record for reputation calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub trade_id: String,
    pub timestamp: u64,
    pub amount: u64,
    pub currency: String,
    pub counterparty: String,
    pub outcome: TradeOutcome,
    pub geographic_distance: Option<f32>,
    pub htlc_used: bool,
    pub dispute_resolved: bool,
}

/// Trade outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeOutcome {
    /// Trade completed successfully
    Success,
    /// Trade failed due to peer
    PeerFailure,
    /// Trade failed due to external factors
    ExternalFailure,
    /// Trade disputed (currently in dispute)
    Disputed,
    /// Trade resolved in peer's favor
    ResolvedInFavor,
    /// Trade resolved against peer
    ResolvedAgainst,
}

/// Location record for geographic clustering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRecord {
    pub timestamp: u64,
    pub location: GeoLocation,
    pub duration_hours: u32,
    pub activity_level: u8, // 0-255
}

/// Activity patterns for behavior analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPatterns {
    pub total_trades: u64,
    pub average_trades_per_day: f32,
    pub peak_activity_hours: Vec<u8>,
    pub preferred_currencies: Vec<String>,
    pub trade_size_distribution: SizeDistribution,
    pub response_time_avg_ms: u32,
    pub cancellation_rate: f32,
}

/// Trade size distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeDistribution {
    pub small_trades: u32,  // < 100 USD
    pub medium_trades: u32, // 100-1000 USD
    pub large_trades: u32,  // > 1000 USD
}

/// Reputation decay data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayData {
    pub last_activity: u64,
    pub decay_rate: f32,
    pub decay_factor: f32,
    pub inactive_days: u32,
    pub recovery_rate: f32,
}

/// Geographic cluster data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoCluster {
    pub cluster_id: String,
    pub region: String,
    pub member_peers: Vec<String>,
    pub total_trades: u32,
    pub average_score: f32,
    pub trust_level: f32,
    pub last_activity: u64,
}

/// Scoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    /// Base score for new peers
    pub base_new_peer_score: u16,
    
    /// Maximum score possible
    pub max_score: u16,
    
    /// Daily decay rate (percentage)
    pub daily_decay_rate: f32,
    
    /// Recovery rate after activity
    pub recovery_rate: f32,
    
    /// Geographic clustering bonus
    pub geo_cluster_bonus: u16,
    
    /// Trade success weight
    pub trade_success_weight: f32,
    
    /// Trade volume weight
    pub trade_volume_weight: f32,
    
    /// Trust factor weight
    pub trust_factor_weight: f32,
    
    /// Verification level bonuses
    pub verification_bonuses: HashMap<VerificationLevel, u16>,
    
    /// Inactivity penalty threshold (days)
    pub inactivity_threshold: u32,
    
    /// Minimum activity for score maintenance
    pub min_activity_per_month: u32,
}

/// Global scoring statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalScoringStats {
    pub total_peers: u32,
    pub average_score: f32,
    pub score_distribution: ScoreDistribution,
    pub geographic_clusters: u32,
    pub active_peers_24h: u32,
    pub new_peers_24h: u32,
    pub trust_factor_distribution: HashMap<TrustFactorType, u32>,
}

/// Score distribution across ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreDistribution {
    pub excellent: u32, // 900-1000
    pub good: u32,      // 700-899
    pub average: u32,   // 500-699
    pub poor: u32,      // 300-499
    pub very_poor: u32, // 0-299
}

impl AdvancedScoringEngine {
    /// Create new advanced scoring engine
    pub fn new(config: ScoringConfig) -> Self {
        Self {
            peers: HashMap::new(),
            geo_clusters: HashMap::new(),
            config,
            history: BTreeMap::new(),
            global_stats: GlobalScoringStats {
                total_peers: 0,
                average_score: 0.0,
                score_distribution: ScoreDistribution {
                    excellent: 0,
                    good: 0,
                    average: 0,
                    poor: 0,
                    very_poor: 0,
                },
                geographic_clusters: 0,
                active_peers_24h: 0,
                new_peers_24h: 0,
                trust_factor_distribution: HashMap::new(),
            },
        }
    }
    
    /// Register new peer
    pub fn register_peer(&mut self, peer_id: String, verification_level: VerificationLevel) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch - server clock misconfigured")
            .as_secs();
        
        let base_score = self.config.base_new_peer_score + 
            self.config.verification_bonuses.get(&verification_level).unwrap_or(&0);
        
        let peer_data = PeerData {
            peer_id: peer_id.clone(),
            current_score: base_score,
            base_score,
            score_history: vec![ScoreSnapshot {
                timestamp: now,
                score: base_score,
                reason: "Initial registration".to_string(),
                decay_factor: 1.0,
            }],
            trust_factors: vec![],
            trade_history: Vec::new(),
            locations: Vec::new(),
            activity_patterns: ActivityPatterns {
                total_trades: 0,
                average_trades_per_day: 0.0,
                peak_activity_hours: Vec::new(),
                preferred_currencies: Vec::new(),
                trade_size_distribution: SizeDistribution {
                    small_trades: 0,
                    medium_trades: 0,
                    large_trades: 0,
                },
                response_time_avg_ms: 0,
                cancellation_rate: 0.0,
            },
            decay_data: DecayData {
                last_activity: now,
                decay_rate: self.config.daily_decay_rate,
                decay_factor: 1.0,
                inactive_days: 0,
                recovery_rate: self.config.recovery_rate,
            },
            verification_level,
            last_updated: now,
        };
        
        self.peers.insert(peer_id, peer_data);
        self.update_global_stats();
        
        Ok(())
    }
    
    /// Record trade and update peer score
    pub fn record_trade(&mut self, peer_id: &str, trade: TradeRecord) -> Result<()> {
        let peer_data = self.peers.get_mut(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_id))?;
        
        // Add to trade history
        peer_data.trade_history.push(trade.clone());
        
        // Update activity patterns
        Self::update_activity_patterns(peer_data, &trade);
        
        // Calculate and update base score only
        let score_change = Self::calculate_trade_score_change(peer_data, &trade);
        peer_data.base_score = (peer_data.base_score as i32 + score_change)
            .clamp(0, self.config.max_score as i32) as u16;
        
        // Reset decay data (activity)
        peer_data.decay_data.last_activity = trade.timestamp;
        peer_data.decay_data.inactive_days = 0;
        peer_data.decay_data.decay_factor = 1.0;
        
        // Centralized score calculation with all factors
        self.update_peer_score(peer_id)?;
        
        // Update geographic clustering if location available
        if let Some(distance) = trade.geographic_distance {
            self.update_geographic_clustering(peer_id, distance);
        }
        
        Ok(())
    }
    
    /// Add trust factor
    pub fn add_trust_factor(&mut self, peer_id: &str, trust_factor: TrustFactor) -> Result<()> {
        let peer_data = self.peers.get_mut(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_id))?;
        
        // Remove existing factor of same type
        peer_data.trust_factors.retain(|tf| tf.factor_type != trust_factor.factor_type);
        
        // Add new trust factor
        peer_data.trust_factors.push(trust_factor);
        
        // Update current score directly instead of calling update_peer_score
        let peer_score = peer_data.base_score as f32 * 
                          peer_data.decay_data.decay_factor * 
                          peer_data.decay_data.recovery_rate;
        peer_data.current_score = peer_score as u16;
        
        Ok(())
    }
    
    /// Update geographic location
    pub fn update_location(&mut self, peer_id: &str, location: GeoLocation, duration_hours: u32) -> Result<()> {
        let peer_data = self.peers.get_mut(peer_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_id))?;
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch - server clock misconfigured")
            .as_secs();
        
        let location_record = LocationRecord {
            timestamp: now,
            location,
            duration_hours,
            activity_level: 128, // Medium activity
        };
        
        peer_data.locations.push(location_record);
        
        // Update geographic clustering
        self.update_geographic_clustering(peer_id, 0.0);
        
        Ok(())
    }
    
    // Apply temporal decay to all inactive peers
    pub fn apply_temporal_decay(&mut self) -> Result<u32> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut decayed_peers = 0;
        
        // Collect peer IDs to avoid borrowing issues
        let peer_ids: Vec<String> = self.peers.keys().cloned().collect();
        
        for peer_id in &peer_ids {
            let days_inactive = (now - self.peers[peer_id].decay_data.last_activity) / 86400;
            
            if days_inactive > 0 {
                // Calculate decay factor
                let decay_factor = (1.0 - self.peers[peer_id].decay_data.decay_rate).powi(days_inactive as i32);
                
                // Update peer data
                if let Some(peer_data) = self.peers.get_mut(peer_id) {
                    peer_data.decay_data.decay_factor = decay_factor;
                    peer_data.decay_data.inactive_days = days_inactive as u32;
                    
                    decayed_peers += 1;
                }
            }
        }
        
        // Update scores for all decayed peers
        for peer_id in &peer_ids {
            let peer_data = &self.peers[peer_id];
            
            // Only apply recovery_rate if there was actual decay
            let effective_recovery_rate = if peer_data.decay_data.decay_factor < 1.0 {
                peer_data.decay_data.recovery_rate
            } else {
                1.0 // No recovery needed if no decay
            };
            
            let peer_score = peer_data.base_score as f32 * 
                          peer_data.decay_data.decay_factor * 
                          effective_recovery_rate;
            
            if let Some(peer_data) = self.peers.get_mut(peer_id) {
                peer_data.current_score = peer_score as u16;
            }
        }
        
        // Update global statistics
        self.update_global_stats();
        
        Ok(decayed_peers)
    }
    
    /// Get peer score with current decay applied
    pub fn get_peer_score(&self, peer_id: &str) -> Option<u16> {
        self.peers.get(peer_id).map(|peer_data| peer_data.current_score)
    }
    
    /// Get complete peer data
    pub fn get_peer_data(&self, peer_id: &str) -> Option<&PeerData> {
        self.peers.get(peer_id)
    }
    
    /// Get peers by score range
    pub fn get_peers_by_score_range(&self, min_score: u16, max_score: u16) -> Vec<&PeerData> {
        self.peers.values()
            .filter(|peer| peer.current_score >= min_score && peer.current_score <= max_score)
            .collect()
    }
    
    /// Get geographic cluster peers
    pub fn get_cluster_peers(&self, region: &str) -> Vec<&PeerData> {
        self.geo_clusters.get(region)
            .map(|cluster| cluster.member_peers.iter()
                .filter_map(|peer_id| self.peers.get(peer_id))
                .collect())
            .unwrap_or_default()
    }
    
    /// Update peer score with all factors
    fn update_peer_score(&mut self, peer_id: &str) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Scoped borrow to avoid conflicts
        let (final_score, snapshot) = {
            let peer_data = self.peers.get(peer_id)
                .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_id))?;

            // 1. Start with decayed base
            let score = peer_data.base_score as f32 * peer_data.decay_data.decay_factor;

            // 2. Add Trust Factor Bonuses (e.g., +50 for KYC)
            let trust_bonus: f32 = peer_data.trust_factors.iter()
                .map(|tf| tf.value as f32)
                .sum();
            
            // 3. Add Behavioral Bonuses
            let trade_bonus = self.calculate_trade_history_bonus(peer_data) as f32;
            let geo_bonus = self.calculate_geographic_bonus(peer_data) as f32;
            
            let total = (score + trust_bonus + trade_bonus + geo_bonus)
                .min(self.config.max_score as f32)
                .max(0.0);

            (total as u16, ScoreSnapshot {
                timestamp: now,
                score: total as u16,
                reason: "Periodic Update".to_string(),
                decay_factor: peer_data.decay_data.decay_factor,
            })
        };

        if let Some(peer) = self.peers.get_mut(peer_id) {
            peer.current_score = final_score;
            peer.score_history.push(snapshot);
            peer.last_updated = now;
            
            // Keep only last 100 score snapshots
            if peer.score_history.len() > 100 {
                peer.score_history.drain(0..peer.score_history.len() - 100);
            }
        }
        
        Ok(())
    }
    
    /// Calculate trade history bonus
    fn calculate_trade_history_bonus(&self, peer_data: &PeerData) -> u16 {
        let total_trades = peer_data.trade_history.len();
        if total_trades == 0 {
            return 0;
        }
        
        let successful_trades = peer_data.trade_history.iter()
            .filter(|trade| trade.outcome == TradeOutcome::Success)
            .count();
        
        let success_rate = successful_trades as f32 / total_trades as f32;
        
        // Volume bonus
        let total_volume: u64 = peer_data.trade_history.iter()
            .map(|trade| trade.amount)
            .sum();
        
        let volume_bonus = std::cmp::min(total_volume / 1000000, 100) as u16; // 1M = 100 points max
        
        // Success rate bonus
        let success_bonus = (success_rate * 150.0) as u16; // Max 150 points
        
        volume_bonus + success_bonus
    }
    
    /// Calculate geographic clustering bonus using optimized cluster lookup
    fn calculate_geographic_bonus(&self, peer_data: &PeerData) -> u16 {
        let recent_locations = &peer_data.locations;
        if recent_locations.is_empty() {
            return 0;
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before Unix epoch - server clock misconfigured")
            .as_secs();
        
        // Get peer's current region from most recent location
        for location_record in recent_locations.iter().rev() {
            if now - location_record.timestamp > 86400 * 7 { // Only consider last 7 days
                continue;
            }
            
            let location = &location_record.location;
            let region = format!("{}-{}", location.country, 
                location.region.as_deref().unwrap_or("unknown"));
            
            // Use existing cluster lookup instead of O(N^2) search
            if let Some(cluster) = self.geo_clusters.get(&region) {
                if cluster.member_peers.contains(&peer_data.peer_id) && cluster.trust_level > 0.7 {
                    return self.config.geo_cluster_bonus;
                }
            }
            
            // Only check one most recent location for efficiency
            break;
        }
        
        0
    }
    
    /// Haversine distance calculation between two geographic points
    #[allow(dead_code)]
    fn haversine_distance(&self, lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;
        
        let d_lat = (lat2 - lat1).to_radians();
        let d_lon = (lon2 - lon1).to_radians();
        
        let a = (d_lat / 2.0).sin().powi(2) +
                lat1.to_radians().cos() * lat2.to_radians().cos() *
                (d_lon / 2.0).sin().powi(2);
        
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
        
        EARTH_RADIUS_KM * c
    }
    
    /// Calculate activity bonus
    #[allow(dead_code)]
    fn calculate_activity_bonus(&self, peer_data: &PeerData) -> u16 {
        let trades_per_day = peer_data.activity_patterns.average_trades_per_day;
        
        if trades_per_day >= 5.0 {
            50 // Very active
        } else if trades_per_day >= 2.0 {
            30 // Moderately active
        } else if trades_per_day >= 0.5 {
            10 // Lightly active
        } else {
            0
        }
    }
    
    /// Calculate trade score change with antifragile penalties
    fn calculate_trade_score_change(peer_data: &PeerData, trade: &TradeRecord) -> i32 {
        let base_change = match trade.outcome {
            TradeOutcome::Success => 10,
            TradeOutcome::PeerFailure => {
                // Antifragile: Penalize high-score peers much more heavily for failures
                let base_penalty = -25;
                if peer_data.current_score > 800 {
                    base_penalty * 3 // Triple penalty for very high scores
                } else if peer_data.current_score > 600 {
                    base_penalty * 2 // Double penalty for high scores
                } else {
                    base_penalty
                }
            },
            TradeOutcome::ExternalFailure => -5,
            TradeOutcome::Disputed => -10,
            TradeOutcome::ResolvedInFavor => 5,
            TradeOutcome::ResolvedAgainst => -20,
        };
        
        // Volume scaling
        let volume_multiplier = (trade.amount / 10000).min(5) as i32; // Max 5x multiplier
        
        // Geographic proximity bonus
        let geo_bonus = if let Some(dist) = trade.geographic_distance {
            if dist < 10.0 { 2 } else if dist < 50.0 { 1 } else { 0 }
        } else { 0 };
        
        // HTLC usage bonus
        let htlc_bonus = if trade.htlc_used { 3 } else { 0 };
        
        base_change * volume_multiplier + geo_bonus + htlc_bonus
    }
    
    /// Update activity patterns
    fn update_activity_patterns(peer_data: &mut PeerData, trade: &TradeRecord) {
        peer_data.activity_patterns.total_trades += 1;
        
        // Update trade size distribution
        match trade.amount {
            0..=100 => peer_data.activity_patterns.trade_size_distribution.small_trades += 1,
            101..=1000 => peer_data.activity_patterns.trade_size_distribution.medium_trades += 1,
            _ => peer_data.activity_patterns.trade_size_distribution.large_trades += 1,
        }
        
        // Update peak activity hours
        let trade_hour = (trade.timestamp / 3600) % 24;
        peer_data.activity_patterns.peak_activity_hours.push(trade_hour as u8);
        
        // Update average trades per day (avoid division by zero)
        let days_active = if peer_data.activity_patterns.total_trades > 1 {
            let first_trade = peer_data.trade_history.first().unwrap().timestamp;
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
            ((now - first_trade) / 86400).max(1)
        } else {
            1
        };
        
        peer_data.activity_patterns.average_trades_per_day = peer_data.activity_patterns.total_trades as f32 / days_active as f32;
    }
    
    /// Update geographic clustering
    fn update_geographic_clustering(&mut self, peer_id: &str, _distance: f32) {
        if let Some(peer_data) = self.peers.get(peer_id) {
            if peer_data.locations.is_empty() {
                return;
            }
            
            for location_record in &peer_data.locations {
                let location = &location_record.location;
                let region = format!("{}-{}", location.country, 
                    location.region.as_deref().unwrap_or("unknown"));
                
                // Add peer to cluster
                let cluster = self.geo_clusters.entry(region.clone()).or_insert_with(|| GeoCluster {
                    cluster_id: format!("cluster_{}", region),
                    region: region.clone(),
                    member_peers: Vec::new(),
                    total_trades: 0,
                    average_score: 0.0,
                    trust_level: 0.0,
                    last_activity: 0,
                });
                
                if !cluster.member_peers.contains(&peer_id.to_string()) {
                    cluster.member_peers.push(peer_id.to_string());
                }
                
                // Update cluster metrics separately to avoid borrow issues
                let avg_score = cluster.member_peers.iter()
                    .filter_map(|pid| self.peers.get(pid))
                    .map(|pd| pd.current_score as f32)
                    .sum::<f32>() / cluster.member_peers.len() as f32;
                
                cluster.average_score = avg_score;
                cluster.last_activity = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
            }
        }
    }
    
    /// Update cluster metrics
    #[allow(dead_code)]
    fn update_cluster_metrics(&mut self, cluster: &mut GeoCluster) {
        if cluster.member_peers.is_empty() {
            return;
        }
        
        let total_score: u16 = cluster.member_peers.iter()
            .filter_map(|pid| self.peers.get(pid))
            .map(|pd| pd.current_score)
            .sum();
        
        cluster.average_score = total_score as f32 / cluster.member_peers.len() as f32;
        
        // Calculate trust level (percentage of high-trust peers)
        let high_trust_peers = cluster.member_peers.iter()
            .filter_map(|pid| self.peers.get(pid))
            .filter(|pd| pd.current_score > 700)
            .count();
        
        cluster.trust_level = high_trust_peers as f32 / cluster.member_peers.len() as f32;
    }
    
    /// Calculate interaction bonus between two peers
    fn calculate_interaction_bonus(&self, peer_a: &PeerData, peer_b: &PeerData) -> f32 {
        let a_counterparties: std::collections::HashSet<_> = peer_a.trade_history.iter()
            .map(|trade| &trade.counterparty)
            .collect();
        
        let b_counterparties: std::collections::HashSet<_> = peer_b.trade_history.iter()
            .map(|trade| &trade.counterparty)
            .collect();
        
        if a_counterparties.contains(&peer_b.peer_id) || b_counterparties.contains(&peer_a.peer_id) {
            0.05 // 5% bonus for previous interactions
        } else {
            0.0
        }
    }
    
    /// Calculate verification compatibility
    fn calculate_verification_compatibility(&self, peer_a: &PeerData, peer_b: &PeerData) -> f32 {
        match (peer_a.verification_level, peer_b.verification_level) {
            (VerificationLevel::Institutional, VerificationLevel::Institutional) => 0.1,
            (VerificationLevel::Verified, VerificationLevel::Verified) => 0.05,
            (VerificationLevel::Basic, VerificationLevel::Basic) => 0.02,
            _ => 0.0,
        }
    }
    
    /// Update global statistics
    fn update_global_stats(&mut self) {
        self.global_stats.total_peers = self.peers.len() as u32;
        self.global_stats.geographic_clusters = self.geo_clusters.len() as u32;
        
        if self.peers.is_empty() {
            self.global_stats.average_score = 0.0;
            self.global_stats.active_peers_24h = 0;
            return;
        }
        
        let total_score: u16 = self.peers.values()
            .map(|peer| peer.current_score)
            .sum();
        
        self.global_stats.average_score = total_score as f32 / self.peers.len() as f32;
        
        // Count active peers (last 24h)
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let day_ago = now - 86400;
        
        self.global_stats.active_peers_24h = self.peers.values()
            .filter(|peer| peer.decay_data.last_activity > day_ago)
            .count() as u32;
    }
    
    /// Get global statistics
    pub fn get_global_stats(&self) -> &GlobalScoringStats {
        &self.global_stats
    }
    
    /// Calculate trust score between two peers
    pub fn calculate_trust_score(&self, peer_a_id: &str, peer_b_id: &str) -> Result<f32> {
        let peer_a = self.peers.get(peer_a_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_a_id))?;
        
        let peer_b = self.peers.get(peer_b_id)
            .ok_or_else(|| anyhow::anyhow!("Peer not found: {}", peer_b_id))?;
        
        // Calculate interaction bonus
        let interaction_bonus = self.calculate_interaction_bonus(peer_a, peer_b);
        
        // Calculate verification compatibility
        let verification_compatibility = self.calculate_verification_compatibility(peer_a, peer_b);
        
        // Calculate geographic proximity bonus
        let geo_bonus = self.calculate_geographic_bonus(peer_a) as f32;
        
        // Combine all factors
        let base_score = (peer_a.current_score as f32 + peer_b.current_score as f32) / 2.0;
        let final_score = base_score * (1.0 + interaction_bonus + verification_compatibility + geo_bonus);
        
        Ok(final_score / 1000.0) // Normalize to 0-1 range
    }
}

impl Default for ScoringConfig {
    fn default() -> Self {
        let mut verification_bonuses = HashMap::new();
        verification_bonuses.insert(VerificationLevel::None, 0);
        verification_bonuses.insert(VerificationLevel::Basic, 50);
        verification_bonuses.insert(VerificationLevel::Verified, 150);
        verification_bonuses.insert(VerificationLevel::Institutional, 300);
        
        Self {
            base_new_peer_score: 500,
            max_score: 1000,
            daily_decay_rate: 0.01, // 1% per day
            recovery_rate: 0.1, // 10% recovery on activity
            geo_cluster_bonus: 25,
            trade_success_weight: 0.4,
            trade_volume_weight: 0.3,
            trust_factor_weight: 0.2,
            verification_bonuses,
            inactivity_threshold: 30, // 30 days
            min_activity_per_month: 5, // 5 trades per month
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwa::market::VerificationLevel;
    
    #[test]
    fn test_peer_registration() {
        let mut engine = AdvancedScoringEngine::new(ScoringConfig::default());
        
        engine.register_peer("peer1".to_string(), VerificationLevel::Basic).unwrap();
        
        let score = engine.get_peer_score("peer1").unwrap();
        assert_eq!(score, 550); // 500 base + 50 verification bonus
    }
    
    #[test]
    fn test_trade_recording() {
        let mut engine = AdvancedScoringEngine::new(ScoringConfig::default());
        
        engine.register_peer("peer1".to_string(), VerificationLevel::Basic).unwrap();
        
        let trade = TradeRecord {
            trade_id: "trade1".to_string(),
            timestamp: 1234567890,
            amount: 50000, // 500 USD
            currency: "USD".to_string(),
            counterparty: "peer2".to_string(),
            outcome: TradeOutcome::Success,
            geographic_distance: Some(50.0),
            htlc_used: true,
            dispute_resolved: false,
        };
        
        engine.record_trade("peer1", trade).unwrap();
        
        let peer_data = engine.get_peer_data("peer1").unwrap();
        assert_eq!(peer_data.trade_history.len(), 1);
        assert!(peer_data.current_score > 550); // Should have increased
    }
    
    #[test]
    fn test_temporal_decay() {
        let mut engine = AdvancedScoringEngine::new(ScoringConfig::default());
        
        engine.register_peer("peer1".to_string(), VerificationLevel::Basic).unwrap();
        
        // Simulate 10 days of inactivity
        let peer_data = engine.peers.get_mut("peer1").unwrap();
        peer_data.decay_data.last_activity = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() - 86400 * 10;
        
        let decayed_peers = engine.apply_temporal_decay().unwrap();
        assert_eq!(decayed_peers, 1);
        
        let current_score = engine.get_peer_score("peer1").unwrap();
        assert!(current_score < 550); // Should have decayed
    }
    
    #[test]
    fn test_trust_score_calculation() {
        let mut engine = AdvancedScoringEngine::new(ScoringConfig::default());
        
        engine.register_peer("peer1".to_string(), VerificationLevel::Verified).unwrap();
        engine.register_peer("peer2".to_string(), VerificationLevel::Verified).unwrap();
        
        let trust_score = engine.calculate_trust_score("peer1", "peer2").unwrap();
        assert!(trust_score > 0.5); // Should be reasonable trust score
    }
}
