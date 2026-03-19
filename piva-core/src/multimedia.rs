//! # Optimized Multimedia Storage
//!
//! Specialized storage for audio/video content with chunk-based access and mobile optimization.

use redb::{Database, ReadableTable};
use serde::{Serialize, Deserialize};
use thiserror::Error;
use std::sync::Arc;
use crate::rwa::VerifiedChunk;
use crate::made::now_secs;
use crate::cache::ChunkCache;

#[derive(Error, Debug)]
pub enum MultimediaError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Chunk not found: {0}")]
    ChunkNotFound(String),
    #[error("Storage full: {0}")]
    StorageFull(String),
    #[error("Invalid chunk data: {0}")]
    InvalidChunk(String),
}

/// Multimedia storage configuration
#[derive(Debug, Clone)]
pub struct MultimediaConfig {
    /// Maximum cache size in bytes
    pub max_cache_size: u64,
    /// Chunk size for mobile optimization
    pub mobile_chunk_size: u32,
    /// Compression level (0-9)
    pub compression_level: u8,
    /// Prefetch enabled
    pub prefetch_enabled: bool,
    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Random eviction
    Random,
    /// Size-based eviction (largest first)
    SizeBased,
}

impl Default for MultimediaConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 100 * 1024 * 1024, // 100MB
            mobile_chunk_size: 64 * 1024, // 64KB for mobile
            compression_level: 6,
            prefetch_enabled: true,
            eviction_policy: EvictionPolicy::LRU,
        }
    }
}

/// Chunk metadata for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Asset ID this chunk belongs to
    pub asset_id: [u8; 32],
    /// Chunk index
    pub chunk_index: u32,
    /// Chunk size in bytes
    pub chunk_size: u32,
    /// Compression used
    pub compression: CompressionType,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Access frequency
    pub access_count: u64,
    /// CRC32 checksum
    pub checksum: u32,
}

/// Compression types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Zlib,
    Lz4,
    Snappy,
}

/// Optimized multimedia storage
pub struct MultimediaStorage {
    db: Arc<Database>,
    config: MultimediaConfig,
    /// In-memory cache for hot chunks
    cache: Arc<ChunkCache>,
}

impl MultimediaStorage {
    /// Create new multimedia storage instance
    pub fn new(db: Database, config: MultimediaConfig) -> Result<Self, MultimediaError> {
        let cache = Arc::new(ChunkCache::new(config.max_cache_size));
        
        Ok(Self {
            db: Arc::new(db),
            config,
            cache,
        })
    }

    /// Store chunk with mobile optimization
    pub fn store_chunk(&self, asset_id: [u8; 32], chunk: &VerifiedChunk) -> Result<(), MultimediaError> {
        // Optimize chunk size for mobile if needed
        let optimized_chunk = self.optimize_for_mobile(chunk)?;
        
        // Compress chunk if enabled
        let compressed_data = self.compress_chunk(&optimized_chunk.data)?;
        
        // Create metadata
        let metadata = ChunkMetadata {
            asset_id,
            chunk_index: chunk.index,
            chunk_size: compressed_data.len() as u32,
            compression: self.get_compression_type(),
            last_accessed: now_secs(),
            access_count: 1,
            checksum: self.calculate_checksum(&compressed_data),
        };

        // Store in database
        let write_txn = self.db.begin_write()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        {
            let mut table = write_txn.open_table(crate::tables::MULTIMEDIA_TABLE)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            let chunk_key = self.create_chunk_key(asset_id, chunk.index);
            let chunk_value = ChunkStorageValue {
                data: compressed_data.clone(),
                metadata,
            };
            
            let serialized = bincode::serialize(&chunk_value)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            table.insert(&chunk_key[..], &*serialized)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        }
        
        write_txn.commit()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;

        // Add to cache if space available
        if self.should_cache(&compressed_data) {
            self.cache.insert(asset_id, chunk.index, optimized_chunk.data.clone());
        }

        Ok(())
    }

    /// Retrieve chunk with streaming support
    pub fn get_chunk(&self, asset_id: [u8; 32], chunk_index: u32) -> Result<VerifiedChunk, MultimediaError> {
        // Check cache first
        if let Some(cached_data) = self.cache.get(asset_id, chunk_index) {
            return Ok(VerifiedChunk {
                index: chunk_index,
                data: cached_data.clone(),
                bao_proof: vec![], // Would need to store this separately
                hash: hash(&cached_data),
                verified_at: now_secs(),
            });
        }

        // Retrieve from database
        let read_txn = self.db.begin_read()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        let table = read_txn.open_table(crate::tables::MULTIMEDIA_TABLE)
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        let chunk_key = self.create_chunk_key(asset_id, chunk_index);
        let stored_value = table.get(&chunk_key[..])
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?
            .ok_or_else(|| MultimediaError::ChunkNotFound(format!("Chunk {} not found", chunk_index)))?;
        
        let chunk_value: ChunkStorageValue = bincode::deserialize(&stored_value.value())
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;

        // Verify checksum
        let computed_checksum = self.calculate_checksum(&chunk_value.data);
        if computed_checksum != chunk_value.metadata.checksum {
            return Err(MultimediaError::InvalidChunk("Checksum mismatch".to_string()));
        }

        // Decompress data
        let decompressed_data = self.decompress_chunk(&chunk_value.data, &chunk_value.metadata.compression)?;
        
        // Update access statistics
        self.update_access_stats(asset_id, chunk_index)?;

        // Add to cache
        if self.should_cache(&chunk_value.data) {
            self.cache.insert(asset_id, chunk_index, decompressed_data.clone());
        }

        Ok(VerifiedChunk {
            index: chunk_index,
            data: decompressed_data,
            bao_proof: vec![], // Would need to store this separately
            hash: hash(&chunk_value.data),
            verified_at: now_secs(),
        })
    }

    /// Stream chunks sequentially for playback
    pub fn stream_chunks(&self, asset_id: [u8; 32], start_index: u32, count: u32) -> Result<Vec<VerifiedChunk>, MultimediaError> {
        let mut chunks = Vec::new();
        
        for i in start_index..(start_index + count) {
            match self.get_chunk(asset_id, i) {
                Ok(chunk) => chunks.push(chunk),
                Err(MultimediaError::ChunkNotFound(_)) => break, // End of asset
                Err(e) => return Err(e),
            }
        }
        
        Ok(chunks)
    }

    /// Prefetch next chunks for smooth playback
    pub fn prefetch_chunks(&self, asset_id: [u8; 32], current_index: u32, prefetch_count: u32) -> Result<(), MultimediaError> {
        if !self.config.prefetch_enabled {
            return Ok(());
        }

        let prefetch_start = current_index + 1;
        
        for i in prefetch_start..(prefetch_start + prefetch_count) {
            // Just access the chunk to load it into cache
            if let Err(MultimediaError::ChunkNotFound(_)) = self.get_chunk(asset_id, i) {
                break; // End of asset
            }
        }

        Ok(())
    }

    /// Get storage statistics
    pub fn get_storage_stats(&self) -> Result<StorageStats, MultimediaError> {
        let read_txn = self.db.begin_read()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        let table = read_txn.open_table(crate::tables::MULTIMEDIA_TABLE)
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        let mut total_size = 0u64;
        let mut chunk_count = 0u64;
        
        // 1. Mapeamos el error de apertura del iterador (Fix E0277)
        let iterator = table.iter().map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;

        for step_result in iterator {
            // 2. Obtenemos la entrada (Key, Value) - Fix E0599
            let (_key_guard, value_guard) = step_result.map_err(|e| {
                MultimediaError::DatabaseError(e.to_string())
            })?;
            
            // 3. Accedemos directamente al valor (ya es AccessGuard)
            let chunk_value: ChunkStorageValue = bincode::deserialize(&value_guard.value())
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            total_size += chunk_value.data.len() as u64;
            chunk_count += 1;
        }

        let cache_stats = self.cache.get_stats();

        Ok(StorageStats {
            total_chunks: chunk_count,
            total_size_bytes: total_size,
            cache_size_bytes: cache_stats.size_bytes,
            cache_hit_rate: cache_stats.hit_rate,
            compression_ratio: self.calculate_compression_ratio(),
        })
    }

    /// Optimize chunk for mobile devices
    fn optimize_for_mobile(&self, chunk: &VerifiedChunk) -> Result<VerifiedChunk, MultimediaError> {
        if chunk.data.len() <= self.config.mobile_chunk_size as usize {
            return Ok(chunk.clone());
        }

        // Split large chunks for mobile
        let mobile_chunk = VerifiedChunk {
            index: chunk.index,
            data: chunk.data[..self.config.mobile_chunk_size as usize].to_vec(),
            bao_proof: chunk.bao_proof.clone(),
            hash: hash(&chunk.data[..self.config.mobile_chunk_size as usize]),
            verified_at: chunk.verified_at,
        };

        Ok(mobile_chunk)
    }

    /// Compress chunk data
    fn compress_chunk(&self, data: &[u8]) -> Result<Vec<u8>, MultimediaError> {
        if self.config.compression_level == 0 {
            return Ok(data.to_vec());
        }

        // Simplified compression - in real implementation use proper compression library
        match self.get_compression_type() {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zlib => {
                // Placeholder for zlib compression
                Ok(data.to_vec())
            }
            CompressionType::Lz4 => {
                // Placeholder for LZ4 compression
                Ok(data.to_vec())
            }
            CompressionType::Snappy => {
                // Placeholder for Snappy compression
                Ok(data.to_vec())
            }
        }
    }

    /// Decompress chunk data
    fn decompress_chunk(&self, data: &[u8], compression: &CompressionType) -> Result<Vec<u8>, MultimediaError> {
        match compression {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zlib => {
                // Placeholder for zlib decompression
                Ok(data.to_vec())
            }
            CompressionType::Lz4 => {
                // Placeholder for LZ4 decompression
                Ok(data.to_vec())
            }
            CompressionType::Snappy => {
                // Placeholder for Snappy decompression
                Ok(data.to_vec())
            }
        }
    }

    /// Get compression type based on config
    fn get_compression_type(&self) -> CompressionType {
        if self.config.compression_level == 0 {
            CompressionType::None
        } else if self.config.compression_level <= 3 {
            CompressionType::Snappy
        } else if self.config.compression_level <= 6 {
            CompressionType::Lz4
        } else {
            CompressionType::Zlib
        }
    }

    /// Calculate CRC32 checksum
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        // Simplified checksum - in real implementation use proper CRC32
        data.iter().fold(0u32, |acc, &byte| acc.wrapping_mul(31).wrapping_add(byte as u32))
    }

    /// Create chunk key for database storage
    fn create_chunk_key(&self, asset_id: [u8; 32], chunk_index: u32) -> [u8; 36] {
        let mut key = [0u8; 36];
        key[..32].copy_from_slice(&asset_id);
        key[32..36].copy_from_slice(&chunk_index.to_le_bytes());
        key
    }

    /// Check if chunk should be cached
    fn should_cache(&self, data: &[u8]) -> bool {
        data.len() as u64 <= self.config.max_cache_size / 10 // Cache if smaller than 10% of max size
    }

    /// Update access statistics for a chunk
    fn update_access_stats(&self, asset_id: [u8; 32], chunk_index: u32) -> Result<(), MultimediaError> {
        let write_txn = self.db.begin_write()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        
        let chunk_key = self.create_chunk_key(asset_id, chunk_index);
        
        // First, get the current value
        let current_value = {
            let read_txn = self.db.begin_read()
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            let table = read_txn.open_table(crate::tables::MULTIMEDIA_TABLE)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            table.get(&chunk_key[..])
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?
                .ok_or_else(|| MultimediaError::ChunkNotFound(format!("Chunk {} not found", chunk_index)))?
        };
        
        // Then update and write back
        {
            let mut table = write_txn.open_table(crate::tables::MULTIMEDIA_TABLE)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            let mut chunk_value: ChunkStorageValue = bincode::deserialize(&current_value.value())
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            chunk_value.metadata.last_accessed = now_secs();
            chunk_value.metadata.access_count += 1;
            
            let serialized = bincode::serialize(&chunk_value)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
            
            table.insert(&chunk_key[..], &*serialized)
                .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;
        }
        
        write_txn.commit()
            .map_err(|e| MultimediaError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Calculate compression ratio
    fn calculate_compression_ratio(&self) -> f32 {
        // Placeholder - would need to track original vs compressed sizes
        0.7 // Assume 30% compression
    }
}

/// Storage value for chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkStorageValue {
    data: Vec<u8>,
    metadata: ChunkMetadata,
}

/// Storage statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_chunks: u64,
    pub total_size_bytes: u64,
    pub cache_size_bytes: u64,
    pub cache_hit_rate: f32,
    pub compression_ratio: f32,
}

/// Calculate hash
fn hash(data: &[u8]) -> [u8; 32] {
    // Placeholder - would use actual hash function
    let mut hash = [0u8; 32];
    let len = std::cmp::min(data.len(), 32);
    hash[..len].copy_from_slice(&data[..len]);
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::MULTIMEDIA_TABLE;

    #[test]
    fn test_chunk_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_multimedia.redb");
        let db = redb::Database::create(db_path).unwrap();
        
        // Create table
        let write_txn = db.begin_write().unwrap();
        {
            let _ = write_txn.open_table(MULTIMEDIA_TABLE);
        }
        write_txn.commit().unwrap();
        
        let config = MultimediaConfig::default();
        let storage = MultimediaStorage::new(db, config).unwrap();
        
        // Create test chunk
        let chunk = VerifiedChunk {
            index: 0,
            data: b"Test audio chunk data for multimedia storage".to_vec(),
            bao_proof: vec![1, 2, 3],
            hash: hash(b"Test audio chunk data for multimedia storage"),
            verified_at: now_secs(),
        };
        
        let asset_id = [1u8; 32];
        
        // Store chunk
        storage.store_chunk(asset_id, &chunk).unwrap();
        
        // Retrieve chunk
        let retrieved = storage.get_chunk(asset_id, 0).unwrap();
        assert_eq!(retrieved.index, 0);
        assert_eq!(retrieved.data, chunk.data);
    }

    #[test]
    fn test_mobile_optimization() {
        let config = MultimediaConfig {
            mobile_chunk_size: 32,
            ..MultimediaConfig::default()
        };
        
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test_mobile.redb");
        let db = redb::Database::create(db_path).unwrap();
        
        let storage = MultimediaStorage::new(db, config).unwrap();
        
        // Create large chunk
        let large_chunk = VerifiedChunk {
            index: 0,
            data: vec![0u8; 100], // 100 bytes
            bao_proof: vec![1, 2, 3],
            hash: hash(&vec![0u8; 100]),
            verified_at: now_secs(),
        };
        
        let optimized = storage.optimize_for_mobile(&large_chunk).unwrap();
        assert_eq!(optimized.data.len(), 32); // Should be truncated to mobile chunk size
    }
}
