//! # Scoring Storage Integration
//!
//! Integrates the Proof of Productivity scoring system with redb storage.

use crate::tables::PEER_SCORES_TABLE;
use piva_core::scoring::{PeerScore, Achievement, ScoringError};
use redb::{Database, ReadableTable};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Storage extension for peer scoring
pub struct ScoringStorage {
    db: Database,
    aggregator: piva_core::scoring::AchievementAggregator,
}

impl ScoringStorage {
    /// Create new scoring storage instance
    pub fn new(db: Database) -> Result<Self, ScoringError> {
        // FORZAR CREACIÓN FÍSICA de la tabla
        let write_txn = db.begin_write().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        {
            // Esto crea la tabla si no existe
            let _ = write_txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        }
        write_txn.commit().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        Ok(Self {
            db,
            aggregator: piva_core::scoring::AchievementAggregator::new(),
        })
    }
    
    /// Get raw database reference for testing purposes
    pub fn raw_db(&self) -> &redb::Database {
        &self.db
    }

    /// Check if there are pending achievements to flush
    pub fn has_pending_updates(&self) -> bool {
        self.aggregator.has_pending()
    }
    
    /// Get or create peer score
    pub fn get_peer_score(&self, peer_id: &[u8; 32]) -> Result<PeerScore, ScoringError> {
        let txn = self.db.begin_read().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        let table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        match table.get(peer_id.as_slice()).map_err(|e| ScoringError::DatabaseError(e.to_string()))? {
            Some(guard) => {
                let score: PeerScore = bincode::deserialize(guard.value())
                    .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
                Ok(score)
            }
            None => Ok(PeerScore::new(*peer_id)),
        }
    }
    
    /// Save peer score to database
    pub fn save_peer_score(&self, peer_id: &[u8; 32], score: &PeerScore) -> Result<(), ScoringError> {
        let txn = self.db.begin_write().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        {
            let mut table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            let serialized = bincode::serialize(score)
                .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            table.insert(peer_id.as_slice(), serialized.as_slice()).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        }
        txn.commit().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        Ok(())
    }
    
    /// Record achievement for a peer
    pub fn record_achievement(&mut self, peer_id: &[u8; 32], achievement: Achievement) -> Result<(), ScoringError> {
        // Add to aggregator
        self.aggregator.add_achievement(*peer_id, achievement);
        
        // If aggregator has pending achievements, flush them
        if self.aggregator.has_pending() {
            self.flush_achievements()?;
        }
        
        Ok(())
    }
    
    /// Flush pending achievements to database
    pub fn flush_achievements(&mut self) -> Result<(), ScoringError> {
        let pending = self.aggregator.flush();
        
        if pending.is_empty() {
            return Ok(());
        }
        
        let txn = self.db.begin_write().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        {
            let mut table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            
            for (peer_id, achievement) in pending {
                // Get existing score or create new
                let mut score = self.get_peer_score(&peer_id)?;
                
                // Record achievement
                score.record_achievement(achievement);
                
                // Save updated score
                let serialized = bincode::serialize(&score)
                    .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
                table.insert(peer_id.as_slice(), serialized.as_slice()).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            }
        }
        txn.commit().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Get top N peers by score
    pub fn get_top_peers(&self, limit: usize) -> Result<Vec<PeerScore>, ScoringError> {
        let txn = self.db.begin_read().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        let table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        let mut scores = Vec::new();
        let iter = table.iter().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        for item in iter {
            let (_key_guard, value_guard) = item.map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            let score: PeerScore = bincode::deserialize(value_guard.value())
                .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            scores.push(score);
        }
        
        // Sort by total score descending
        scores.sort_by(|a, b| b.total_score.cmp(&a.total_score));
        
        // Return top N
        Ok(scores.into_iter().take(limit).collect())
    }
    
    /// Get peers by trust level
    pub fn get_peers_by_trust_level(&self, trust_level: piva_core::scoring::TrustLevel) -> Result<Vec<PeerScore>, ScoringError> {
        let txn = self.db.begin_read().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        let table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        let mut matching_peers = Vec::new();
        let iter = table.iter().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        for item in iter {
            let (_key_guard, value_guard) = item.map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            let score: PeerScore = bincode::deserialize(value_guard.value())
                .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            
            if score.trust_level() == trust_level {
                matching_peers.push(score);
            }
        }
        
        Ok(matching_peers)
    }
    
    /// Get network statistics
    pub fn get_network_stats(&self) -> Result<NetworkStats, ScoringError> {
        let txn = self.db.begin_read().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        let table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        let mut stats = NetworkStats::default();
        let iter = table.iter().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        
        for item in iter {
            let (_key_guard, value_guard) = item.map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            let score: PeerScore = bincode::deserialize(value_guard.value())
                .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            
            stats.total_peers += 1;
            stats.total_score += score.total_score as i64;
            
            // Count by trust level
            match score.trust_level() {
                piva_core::scoring::TrustLevel::Excellent => stats.excellent_peers += 1,
                piva_core::scoring::TrustLevel::Good => stats.good_peers += 1,
                piva_core::scoring::TrustLevel::Neutral => stats.neutral_peers += 1,
                piva_core::scoring::TrustLevel::Poor => stats.poor_peers += 1,
                piva_core::scoring::TrustLevel::VeryPoor => stats.very_poor_peers += 1,
                piva_core::scoring::TrustLevel::Banned => stats.banned_peers += 1,
            }
            
            // Update min/max scores
            stats.min_score = stats.min_score.min(score.total_score);
            stats.max_score = stats.max_score.max(score.total_score);
            
            // Calculate average latency
            stats.average_latency = 
                (stats.average_latency * (stats.total_peers - 1) as f32 + score.average_latency_ms) / stats.total_peers as f32;
        }
        
        if stats.total_peers > 0 {
            stats.average_score = (stats.total_score / stats.total_peers as i64) as i32;
        }
        
        Ok(stats)
    }
    
    /// Cleanup old scores (peers not seen in N days)
    pub fn cleanup_old_scores(&self, days_threshold: u64) -> Result<usize, ScoringError> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() - (days_threshold * 24 * 3600);
        
        let txn = self.db.begin_write().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        let mut removed = 0;
        
        {
            let mut table = txn.open_table(PEER_SCORES_TABLE).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            
            // Collect peers to remove first
            let mut peers_to_remove = Vec::new();
            let iter = table.iter().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
            
            for item in iter {
                let (key_guard, value_guard) = item.map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
                let score: PeerScore = bincode::deserialize(value_guard.value())
                    .map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
                
                if score.last_seen < cutoff_time {
                    peers_to_remove.push(key_guard.value().to_vec());
                }
            }
            
            // Remove peers
            for peer_id in peers_to_remove {
                table.remove(peer_id.as_slice()).map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
                removed += 1;
            }
        }
        
        txn.commit().map_err(|e| ScoringError::DatabaseError(e.to_string()))?;
        Ok(removed)
    }
}

/// Network statistics for scoring system
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_peers: u64,
    pub total_score: i64,
    pub average_score: i32,
    pub min_score: i32,
    pub max_score: i32,
    pub average_latency: f32,
    
    // Count by trust level
    pub excellent_peers: u64,
    pub good_peers: u64,
    pub neutral_peers: u64,
    pub poor_peers: u64,
    pub very_poor_peers: u64,
    pub banned_peers: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use piva_core::network::NetworkMode;
    use tempfile::TempDir;
    
    #[test]
    fn test_scoring_storage_basic() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open_disk(temp_dir.path().join("test"), NetworkMode::Devnet)?;
        let db = storage.db.clone();
        
        let scoring = ScoringStorage::new(db);
        
        // Create a peer score
        let peer_id = [1u8; 32];
        let mut score = PeerScore::new(peer_id);
        
        // Record an achievement
        let achievement = Achievement::ChunkDelivery {
            chunk_hash: [2u8; 32],
            delivery_time_ms: 100,
        };
        
        score.record_achievement(achievement);
        scoring.save_peer_score(&peer_id, &score)?;
        
        // Retrieve the score
        let retrieved = scoring.get_peer_score(&peer_id)?;
        assert_eq!(retrieved.total_score, 501);
        assert_eq!(retrieved.successful_deliveries, 1);
        
        Ok(())
    }
    
    #[test]
    fn test_achievement_aggregator() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open_disk(temp_dir.path().join("test"), NetworkMode::Devnet)?;
        let db = storage.db.clone();
        
        let mut scoring = ScoringStorage::new(db);
        let peer_id = [1u8; 32];
        
        // Add multiple achievements
        for i in 0..5 {
            let achievement = Achievement::ChunkDelivery {
                chunk_hash: [i; 32],
                delivery_time_ms: 100 + i as u64,
            };
            scoring.record_achievement(&peer_id, achievement)?;
        }
        
        // Force flush
        scoring.flush_achievements()?;
        
        // Verify all achievements were recorded
        let score = scoring.get_peer_score(&peer_id)?;
        assert_eq!(score.successful_deliveries, 5);
        assert_eq!(score.total_score, 505); // 500 + 5
        
        Ok(())
    }
    
    #[test]
    fn test_trust_level_filtering() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let storage = Storage::open_disk(temp_dir.path().join("test"), NetworkMode::Devnet)?;
        let db = storage.db.clone();
        
        let scoring = ScoringStorage::new(db);
        
        // Create peers with different scores
        let excellent_peer = PeerScore {
            peer_id: [1u8; 32],
            total_score: 900,
            ..Default::default()
        };
        
        let banned_peer = PeerScore {
            peer_id: [2u8; 32],
            total_score: -100,
            ..Default::default()
        };
        
        scoring.save_peer_score(&excellent_peer.peer_id, &excellent_peer)?;
        
        scoring.save_peer_score(&banned_peer.peer_id, &banned_peer)?;
        
        // Filter by trust level
        let excellent_peers = scoring.get_peers_by_trust_level(piva_core::scoring::TrustLevel::Excellent)?;
        assert_eq!(excellent_peers.len(), 1);
        assert_eq!(excellent_peers[0].peer_id, [1u8; 32]);
        
        let banned_peers = scoring.get_peers_by_trust_level(piva_core::scoring::TrustLevel::Banned)?;
        assert_eq!(banned_peers.len(), 1);
        assert_eq!(banned_peers[0].peer_id, [2u8; 32]);
        
        Ok(())
    }
}
