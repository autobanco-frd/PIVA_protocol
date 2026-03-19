//! # MADE - Micro-Agent for Decision Engine
//!
//! Agente proactivo ligero sin LLMs para gestión autónoma de recursos y red.
//! Implementa decisiones estocásticas basadas en heurísticas simples.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::sync::atomic::{AtomicBool, Ordering};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use rand::Rng;

#[derive(Error, Debug)]
pub enum MadeError {
    #[error("Resource monitoring failed: {0}")]
    ResourceError(String),
    #[error("Decision failed: {0}")]
    DecisionError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
}

/// Resource usage metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub ram_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub cpu_usage_percent: f32,
    pub network_latency_ms: u64,
    pub active_connections: usize,
    pub timestamp: u64,
}

impl Default for ResourceMetrics {
    fn default() -> Self {
        Self {
            ram_usage_percent: 0.0,
            disk_usage_percent: 0.0,
            cpu_usage_percent: 0.0,
            network_latency_ms: 0,
            active_connections: 0,
            timestamp: now_secs(),
        }
    }
}

/// Decision types for autonomous actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MadeDecision {
    /// Prune low-reputation assets to free resources
    PruneAssets { 
        count: usize, 
        reason: String 
    },
    /// Seek new high-score peers if connection lost
    SeekPeers { 
        min_score: i32,
        max_attempts: usize 
    },
    /// Pre-fetch metadata based on user interests
    PrefetchMetadata { 
        asset_ids: Vec<[u8; 32]>,
        priority: u8 
    },
    /// Adjust performance parameters
    AdjustPerformance { 
        new_interval_ms: u64,
        reason: String 
    },
    /// No action needed
    NoAction { 
        reason: String 
    },
}

/// Autonomous decision engine
pub struct MadeAgent {
    /// Current resource metrics
    metrics: Arc<Mutex<ResourceMetrics>>,
    /// Decision history for learning
    decision_history: Arc<Mutex<Vec<(u64, MadeDecision)>>>,
    /// Configuration parameters
    config: MadeConfig,
    /// Running state - using AtomicBool for better performance
    is_running: Arc<AtomicBool>,
}

/// Configuration for MADE agent
#[derive(Debug, Clone)]
pub struct MadeConfig {
    /// Tick interval in seconds
    pub tick_interval_secs: u64,
    /// Resource thresholds (0.0 to 1.0)
    pub ram_threshold: f32,
    pub disk_threshold: f32,
    pub cpu_threshold: f32,
    /// Decision parameters
    pub max_prune_batch: usize,
    pub min_peer_score: i32,
    /// Stochastic decision probability
    pub decision_probability: f32,
}

impl Default for MadeConfig {
    fn default() -> Self {
        Self {
            tick_interval_secs: 30, // 30 seconds
            ram_threshold: 0.8,    // 80%
            disk_threshold: 0.8,   // 80%
            cpu_threshold: 0.7,    // 70%
            max_prune_batch: 10,
            min_peer_score: 300,
            decision_probability: 0.1, // 10% chance of proactive action
        }
    }
}

impl MadeAgent {
    /// Create new MADE agent instance
    pub fn new(config: MadeConfig) -> Self {
        Self {
            metrics: Arc::new(Mutex::new(ResourceMetrics {
                ram_usage_percent: 0.0,
                disk_usage_percent: 0.0,
                cpu_usage_percent: 0.0,
                network_latency_ms: 0,
                active_connections: 0,
                timestamp: now_secs(),
            })),
            decision_history: Arc::new(Mutex::new(Vec::new())),
            config,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Start the autonomous tick loop
    pub fn start(&self) -> Result<(), MadeError> {
        if self.is_running.load(Ordering::Relaxed) {
            return Err(MadeError::DecisionError("Agent already running".to_string()));
        }

        self.is_running.store(true, Ordering::Relaxed);

        let metrics = Arc::clone(&self.metrics);
        let decision_history = Arc::clone(&self.decision_history);
        let config = self.config.clone();
        let is_running = Arc::clone(&self.is_running);

        thread::spawn(move || {
            Self::tick_loop(metrics, decision_history, config, is_running);
        });

        Ok(())
    }

    /// Stop the autonomous tick loop
    pub fn stop(&self) -> Result<(), MadeError> {
        self.is_running.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Main autonomous decision loop
    fn tick_loop(
        metrics: Arc<Mutex<ResourceMetrics>>,
        decision_history: Arc<Mutex<Vec<(u64, MadeDecision)>>>,
        config: MadeConfig,
        is_running: Arc<AtomicBool>,
    ) {
        loop {
            // Check if we should continue running
            if !is_running.load(Ordering::Relaxed) {
                break;
            }

            // Collect current metrics
            let current_metrics = Self::collect_metrics(&config);
            
            // Update shared metrics
            {
                let mut metrics_guard = metrics.lock().unwrap();
                *metrics_guard = current_metrics.clone();
            }

            // Make autonomous decision
            let decision = Self::make_decision(&current_metrics, &config);
            
            // Record decision
            {
                let mut history = decision_history.lock().unwrap();
                history.push((now_secs(), decision.clone()));
                
                // Keep only last 100 decisions
                if history.len() > 100 {
                    history.remove(0);
                }
            }

            // Execute decision (in a real implementation, this would interact with storage/network)
            Self::execute_decision(&decision, &config).unwrap_or_else(|e| {
                eprintln!("Error executing decision: {:?}", e);
            });

            // Sleep until next tick
            thread::sleep(Duration::from_secs(config.tick_interval_secs));
        }
    }

    /// Collect current system metrics
    fn collect_metrics(_config: &MadeConfig) -> ResourceMetrics {
        // In a real implementation, this would use system APIs
        // For now, we'll simulate with placeholder values
        ResourceMetrics {
            ram_usage_percent: 0.45, // 45%
            disk_usage_percent: 0.60, // 60%
            cpu_usage_percent: 0.30,  // 30%
            network_latency_ms: 50,
            active_connections: 5,
            timestamp: now_secs(),
        }
    }

    /// Make autonomous decision based on metrics and stochastic factors
    fn make_decision(metrics: &ResourceMetrics, config: &MadeConfig) -> MadeDecision {
        // Resource pressure analysis
        let ram_pressure = metrics.ram_usage_percent > config.ram_threshold;
        let disk_pressure = metrics.disk_usage_percent > config.disk_threshold;
        let cpu_pressure = metrics.cpu_usage_percent > config.cpu_threshold;

        // Stochastic decision making
        let mut rng = rand::thread_rng();
        let random_factor: f32 = rng.gen(); // 0.0 to 1.0
        
        if (ram_pressure || disk_pressure) && random_factor < config.decision_probability {
            // High resource usage - consider pruning
            MadeDecision::PruneAssets {
                count: config.max_prune_batch,
                reason: format!("Resource pressure: RAM {:.1}%, Disk {:.1}%", 
                    metrics.ram_usage_percent * 100.0, 
                    metrics.disk_usage_percent * 100.0),
            }
        } else if metrics.active_connections == 0 && random_factor < config.decision_probability * 2.0 {
            // No connections - seek new peers
            MadeDecision::SeekPeers {
                min_score: config.min_peer_score,
                max_attempts: 5,
            }
        } else if cpu_pressure && random_factor < config.decision_probability {
            // High CPU - slow down operations
            MadeDecision::AdjustPerformance {
                new_interval_ms: config.tick_interval_secs * 2000, // Double the interval
                reason: format!("CPU pressure: {:.1}%", metrics.cpu_usage_percent * 100.0),
            }
        } else if random_factor < config.decision_probability * 0.5 {
            // Random proactive action - prefetch metadata
            MadeDecision::PrefetchMetadata {
                asset_ids: vec![[1; 32], [2; 32], [3; 32]], // Placeholder asset IDs
                priority: 1,
            }
        } else {
            MadeDecision::NoAction {
                reason: "Normal operation - no action needed".to_string(),
            }
        }
    }

    /// Execute the autonomous decision
    fn execute_decision(decision: &MadeDecision, _config: &MadeConfig) -> Result<(), MadeError> {
        match decision {
            MadeDecision::PruneAssets { count, reason } => {
                println!("🧹 MADE: Pruning {} assets - {}", count, reason);
                // In real implementation, this would:
                // 1. Query storage for low-reputation assets
                // 2. Sort by reputation score
                // 3. Delete the bottom N assets
                Ok(())
            }
            MadeDecision::SeekPeers { min_score, max_attempts } => {
                println!("🔍 MADE: Seeking peers with score >= {} (max {} attempts)", min_score, max_attempts);
                // In real implementation, this would:
                // 1. Query peer discovery system
                // 2. Filter by reputation score
                // 3. Attempt connections
                Ok(())
            }
            MadeDecision::PrefetchMetadata { asset_ids, priority } => {
                println!("📥 MADE: Prefetching {} assets (priority {})", asset_ids.len(), priority);
                // In real implementation, this would:
                // 1. Analyze user interests
                // 2. Query network for metadata
                // 3. Download and cache metadata
                Ok(())
            }
            MadeDecision::AdjustPerformance { new_interval_ms, reason } => {
                println!("⚙️  MADE: Adjusting performance to {}ms - {}", new_interval_ms, reason);
                // In real implementation, this would:
                // 1. Update configuration
                // 2. Restart tick loop with new interval
                Ok(())
            }
            MadeDecision::NoAction { reason } => {
                println!("😌 MADE: {}", reason);
                Ok(())
            }
        }
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> Result<ResourceMetrics, MadeError> {
        let metrics = self.metrics.lock()
            .map_err(|e| MadeError::ResourceError(e.to_string()))?;
        Ok(metrics.clone())
    }

    /// Get decision history
    pub fn get_decision_history(&self) -> Result<Vec<(u64, MadeDecision)>, MadeError> {
        let history = self.decision_history.lock()
            .map_err(|e| MadeError::DecisionError(e.to_string()))?;
        Ok(history.clone())
    }

    /// Check if agent is running
    pub fn is_running(&self) -> Result<bool, MadeError> {
        Ok(self.is_running.load(Ordering::Relaxed))
    }

    /// Make decision based on metrics (for testing)
    pub fn make_decision_direct(&self, metrics: &ResourceMetrics) -> MadeDecision {
        Self::make_decision(metrics, &self.config)
    }
}

/// Helper function to get current timestamp
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_made_agent_creation() {
        let config = MadeConfig::default();
        let agent = MadeAgent::new(config);
        
        assert!(!agent.is_running().unwrap());
        
        let metrics = agent.get_metrics().unwrap();
        assert!(metrics.timestamp > 0); // Should have current timestamp
    }

    #[test]
    fn test_decision_making() {
        let config = MadeConfig {
            decision_probability: 1.0, // Force decision for testing
            ..MadeConfig::default()
        };
        let metrics = ResourceMetrics {
            ram_usage_percent: 0.9, // High RAM usage
            disk_usage_percent: 0.4,
            cpu_usage_percent: 0.3,
            network_latency_ms: 50,
            active_connections: 5,
            timestamp: now_secs(),
        };

        let decision = MadeAgent::make_decision(&metrics, &config);
        
        // Should suggest pruning due to high RAM (with probability 1.0)
        match decision {
            MadeDecision::PruneAssets { .. } => {}, // Expected
            other => panic!("Expected PruneAssets decision, got {:?}", other),
        }
    }
}
