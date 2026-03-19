//! # Chunk Cache for Multimedia Storage
//!
//! In-memory cache for frequently accessed multimedia chunks.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Simple LRU cache for chunks
pub struct ChunkCache {
    /// Maximum cache size in bytes
    max_size: u64,
    /// Current cache size in bytes - using Mutex for interior mutability
    current_size: Arc<Mutex<u64>>,
    /// Cache entries: (asset_id, chunk_index) -> (data, last_accessed, access_count)
    entries: Arc<Mutex<HashMap<([u8; 32], u32), (Vec<u8>, u64, u64)>>>,
    /// Access order for LRU eviction
    access_order: Arc<Mutex<Vec<([u8; 32], u32)>>>,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size_bytes: u64,
    pub hit_rate: f32,
    pub total_requests: u64,
    pub cache_hits: u64,
}

impl ChunkCache {
    /// Create new cache with specified size
    pub fn new(max_size: u64) -> Self {
        Self {
            max_size,
            current_size: Arc::new(Mutex::new(0)),
            entries: Arc::new(Mutex::new(HashMap::new())),
            access_order: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Insert chunk into cache
    pub fn insert(&self, asset_id: [u8; 32], chunk_index: u32, data: Vec<u8>) {
        let key = (asset_id, chunk_index);
        let data_size = data.len() as u64;
        
        // Remove existing entry if present
        self.remove_entry(&key);
        
        // Evict entries if needed
        {
            let mut current_size = self.current_size.lock().unwrap();
            while *current_size + data_size > self.max_size {
                drop(current_size); // Release lock before calling remove_entry
                if let Some(evicted_key) = self.get_lru_key() {
                    self.remove_entry(&evicted_key);
                    current_size = self.current_size.lock().unwrap(); // Reacquire lock
                } else {
                    break;
                }
            }
        }
        
        // Insert new entry
        let now = now_secs();
        {
            let mut entries = self.entries.lock().unwrap();
            entries.insert(key, (data, now, 1));
        }
        
        {
            let mut access_order = self.access_order.lock().unwrap();
            access_order.push(key);
        }
        
        *self.current_size.lock().unwrap() += data_size;
    }

    /// Get chunk from cache
    pub fn get(&self, asset_id: [u8; 32], chunk_index: u32) -> Option<Vec<u8>> {
        let key = (asset_id, chunk_index);
        
        let mut entries = self.entries.lock().unwrap();
        if let Some((data, last_accessed, access_count)) = entries.get_mut(&key) {
            let now = now_secs();
            *last_accessed = now;
            *access_count += 1;
            
            // Update access order
            {
                let mut access_order = self.access_order.lock().unwrap();
                // Remove from current position
                access_order.retain(|&k| k != key);
                // Add to end (most recently used)
                access_order.push(key);
            }
            
            Some(data.clone())
        } else {
            None
        }
    }

    /// Remove entry from cache
    fn remove_entry(&self, key: &([u8; 32], u32)) {
        let mut entries = self.entries.lock().unwrap();
        if let Some((data, _, _)) = entries.remove(key) {
            *self.current_size.lock().unwrap() -= data.len() as u64;
        }
        
        let mut access_order = self.access_order.lock().unwrap();
        access_order.retain(|&k| k != *key);
    }

    /// Get least recently used key for eviction
    fn get_lru_key(&self) -> Option<([u8; 32], u32)> {
        let access_order = self.access_order.lock().unwrap();
        access_order.first().copied()
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let current_size = *self.current_size.lock().unwrap();
        CacheStats {
            size_bytes: current_size,
            hit_rate: 0.0, // Would need to track hits/misses
            total_requests: 0,
            cache_hits: 0,
        }
    }

    /// Clear cache
    pub fn clear(&self) {
        let mut entries = self.entries.lock().unwrap();
        entries.clear();
        
        let mut access_order = self.access_order.lock().unwrap();
        access_order.clear();
        
        *self.current_size.lock().unwrap() = 0;
    }
}

/// Get current timestamp
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
    fn test_cache_insert_retrieve() {
        let cache = ChunkCache::new(1024); // 1KB cache
        
        let asset_id = [1u8; 32];
        let chunk_index = 0;
        let data = b"Test chunk data".to_vec();
        
        // Insert
        cache.insert(asset_id, chunk_index, data.clone());
        
        // Retrieve
        let retrieved = cache.get(asset_id, chunk_index).unwrap();
        assert_eq!(retrieved, data);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = ChunkCache::new(20); // Small cache to test eviction
        
        let asset_id = [1u8; 32];
        
        // Insert first chunk (10 bytes)
        cache.insert(asset_id, 0, vec![0u8; 10]);
        
        // Insert second chunk (15 bytes) - should evict first
        cache.insert(asset_id, 1, vec![0u8; 15]);
        
        // First chunk should be gone
        assert!(cache.get(asset_id, 0).is_none());
        
        // Second chunk should be present
        assert!(cache.get(asset_id, 1).is_some());
    }
}
