use crate::{
    asset::{AssetEntry, AssetMetadata, AssetType},
    network::NetworkMode,
    proto::{AssetEntryProto, AssetMetadataProto, AssetTypeProto, NetworkModeProto},
};
use std::collections::{BTreeMap, HashMap};
use std::convert::{TryFrom, TryInto};

// Convert AssetType to AssetTypeProto
impl From<AssetType> for AssetTypeProto {
    fn from(asset_type: AssetType) -> Self {
        match asset_type {
            AssetType::PropertyTitle => AssetTypeProto::PropertyTitle,
            AssetType::Diploma => AssetTypeProto::Diploma,
            AssetType::LegalDocument => AssetTypeProto::LegalDocument,
            AssetType::CommercialOffer => AssetTypeProto::CommercialOffer,
            AssetType::AudioMusic => AssetTypeProto::AudioMusic,
        }
    }
}

// Convert AssetTypeProto to AssetType
impl TryFrom<AssetTypeProto> for AssetType {
    type Error = String;
    
    fn try_from(proto: AssetTypeProto) -> Result<Self, Self::Error> {
        match proto {
            AssetTypeProto::PropertyTitle => Ok(AssetType::PropertyTitle),
            AssetTypeProto::Diploma => Ok(AssetType::Diploma),
            AssetTypeProto::LegalDocument => Ok(AssetType::LegalDocument),
            AssetTypeProto::CommercialOffer => Ok(AssetType::CommercialOffer),
            AssetTypeProto::AudioMusic => Ok(AssetType::AudioMusic),
        }
    }
}

// Convert NetworkMode to NetworkModeProto
impl From<NetworkMode> for NetworkModeProto {
    fn from(network: NetworkMode) -> Self {
        match network {
            NetworkMode::Devnet => NetworkModeProto::Devnet,
            NetworkMode::Testnet => NetworkModeProto::Testnet,
            NetworkMode::Mainnet => NetworkModeProto::Mainnet,
        }
    }
}

// Convert NetworkModeProto to NetworkMode
impl TryFrom<NetworkModeProto> for NetworkMode {
    type Error = String;
    
    fn try_from(proto: NetworkModeProto) -> Result<Self, Self::Error> {
        match proto {
            NetworkModeProto::Devnet => Ok(NetworkMode::Devnet),
            NetworkModeProto::Testnet => Ok(NetworkMode::Testnet),
            NetworkModeProto::Mainnet => Ok(NetworkMode::Mainnet),
        }
    }
}

// Convert AssetMetadata to AssetMetadataProto
impl From<AssetMetadata> for AssetMetadataProto {
    fn from(metadata: AssetMetadata) -> Self {
        Self {
            asset_type: AssetTypeProto::from(metadata.asset_type) as i32,
            issuer_pubkey: metadata.issuer_pubkey.to_vec(),
            created_at: metadata.created_at,
            description: metadata.description,
            custom_fields: HashMap::from_iter(metadata.custom_fields),
        }
    }
}

// Convert AssetMetadataProto to AssetMetadata
impl TryFrom<AssetMetadataProto> for AssetMetadata {
    type Error = String;
    
    fn try_from(proto: AssetMetadataProto) -> Result<Self, Self::Error> {
        let asset_type_proto = AssetTypeProto::try_from(proto.asset_type)
            .map_err(|_| "Invalid asset type")?;
        let asset_type = AssetType::try_from(asset_type_proto)
            .map_err(|_| "Invalid asset type")?;
        
        // Convert Vec<u8> to [u8; 32] for issuer_pubkey
        let issuer_pubkey: [u8; 32] = proto.issuer_pubkey
            .try_into()
            .map_err(|_| "Invalid issuer public key length")?;
        
        Ok(Self {
            asset_type,
            issuer_pubkey,
            created_at: proto.created_at,
            description: proto.description,
            custom_fields: BTreeMap::from_iter(proto.custom_fields),
        })
    }
}

// Convert AssetEntry to AssetEntryProto
impl From<AssetEntry> for AssetEntryProto {
    fn from(entry: AssetEntry) -> Self {
        Self {
            id: entry.id.to_string(),
            metadata: Some(AssetMetadataProto::from(entry.metadata)),
            content_hash: entry.content_hash.to_vec(),
            content_size: entry.content_size,
            signature: entry.signature.to_vec(),
            network: NetworkModeProto::from(entry.network) as i32,
        }
    }
}

// Convert AssetEntryProto to AssetEntry
impl TryFrom<AssetEntryProto> for AssetEntry {
    type Error = String;
    
    fn try_from(proto: AssetEntryProto) -> Result<Self, Self::Error> {
        let metadata = proto.metadata
            .ok_or("Missing metadata")?
            .try_into()?;
        
        let network_proto = NetworkModeProto::try_from(proto.network)
            .map_err(|_| "Invalid network mode")?;
        let network = NetworkMode::try_from(network_proto)
            .map_err(|_| "Invalid network mode")?;
        
        // Convert Vec<u8> to [u8; 32] for content_hash
        let content_hash: [u8; 32] = proto.content_hash
            .try_into()
            .map_err(|_| "Invalid content hash length")?;
        
        // Convert Vec<u8> to [u8; 64] for signature
        let signature: [u8; 64] = proto.signature
            .try_into()
            .map_err(|_| "Invalid signature length")?;
        
        // Parse AssetId from string
        let id = proto.id.parse()
            .map_err(|e| format!("Invalid asset ID: {}", e))?;
        
        Ok(Self {
            id,
            metadata,
            content_hash,
            content_size: proto.content_size,
            signature,
            network,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::AssetId;
    use piva_crypto::{hash_blake3, KeyPair};
    use prost::Message;
    use std::collections::BTreeMap;

    #[test]
    fn test_asset_entry_roundtrip() {
        // Create test asset entry
        let mut custom_fields = BTreeMap::new();
        custom_fields.insert("test".to_string(), "value".to_string());
        
        let metadata = AssetMetadata {
            asset_type: AssetType::PropertyTitle,
            issuer_pubkey: [1u8; 32],
            created_at: 1234567890,
            description: "Test asset".to_string(),
            custom_fields,
        };
        
        let content_hash = hash_blake3(b"test content");
        let keypair = KeyPair::generate();
        let signature = keypair.sign(&content_hash).to_bytes();
        
        let asset_id = AssetId::from_metadata(&metadata, NetworkMode::Devnet).expect("Failed to create asset ID");
        
        let original = AssetEntry {
            id: asset_id,
            metadata,
            content_hash,
            content_size: 1024,
            signature,
            network: NetworkMode::Devnet,
        };
        
        // Convert to protobuf and back
        let proto = AssetEntryProto::from(original.clone());
        let converted = AssetEntry::try_from(proto).expect("Conversion failed");
        
        // Verify roundtrip
        assert_eq!(original.id, converted.id);
        assert_eq!(original.metadata.asset_type, converted.metadata.asset_type);
        assert_eq!(original.metadata.issuer_pubkey, converted.metadata.issuer_pubkey);
        assert_eq!(original.metadata.created_at, converted.metadata.created_at);
        assert_eq!(original.metadata.description, converted.metadata.description);
        assert_eq!(original.metadata.custom_fields, converted.metadata.custom_fields);
        assert_eq!(original.content_hash, converted.content_hash);
        assert_eq!(original.content_size, converted.content_size);
        assert_eq!(original.signature, converted.signature);
        assert_eq!(original.network, converted.network);
    }
    
    #[test]
    fn test_protobuf_serialization() {
        let metadata = AssetMetadata {
            asset_type: AssetType::Diploma,
            issuer_pubkey: [2u8; 32],
            created_at: 9876543210,
            description: "Test diploma".to_string(),
            custom_fields: BTreeMap::new(),
        };
        
        let content_hash = hash_blake3(b"diploma content");
        let keypair = KeyPair::generate();
        let signature = keypair.sign(&content_hash).to_bytes();
        
        let asset_id = AssetId::from_metadata(&metadata, NetworkMode::Devnet).expect("Failed to create asset ID");
        
        let asset = AssetEntry {
            id: asset_id,
            metadata,
            content_hash,
            content_size: 2048,
            signature,
            network: NetworkMode::Testnet,
        };
        
        // Test protobuf serialization
        let proto = AssetEntryProto::from(asset);
        let bytes = proto.encode_to_vec();
        let decoded = AssetEntryProto::decode(&*bytes).expect("Failed to decode");
        
        // Verify serialization/deserialization
        assert_eq!(proto.id, decoded.id);
        assert_eq!(proto.content_size, decoded.content_size);
        assert_eq!(proto.network, decoded.network);
    }
}
