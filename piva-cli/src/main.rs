//! # PIVA Command Line Interface
//! 
//! Master CLI structure for PIVA protocol with modular handlers

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;
use anyhow::Result;

mod handlers;
mod bridge;

use handlers::{handle_register, handle_list, handle_verify, handle_backup, handle_init, handle_status, handle_config, handle_identity_init, handle_identity_show, handle_identity_invite};
use bridge::evm::{handle_evm_encode, handle_evm_sign};

#[derive(Parser)]
#[command(name = "piva")]
#[command(about = "Protocolo de Intercambio de Valor Antifrágil by FrD")]
#[command(version)]
pub struct Cli {
    /// Network mode (devnet, testnet, mainnet)
    #[arg(short, long, default_value = "devnet")]
    network: String,
    
    /// Data directory for PIVA node
    #[arg(short, long, default_value = "~/.piva")]
    data_dir: Option<String>,
    
    /// Enable debug logging
    #[arg(long)]
    debug: bool,
    
    /// Output in JSON format for AI integration
    #[arg(long)]
    json: bool,
    
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new PIVA node
    Init {
        /// Force initialization even if node already exists
        #[arg(short, long)]
        force: bool,
    },
    
    /// Start the PIVA node
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
        
        /// Enable peer discovery
        #[arg(long)]
        discovery: bool,
    },
    
    /// Asset management commands
    Asset {
        #[command(subcommand)]
        action: AssetAction,
    },
    
    /// Verify and validate assets
    Verify {
        /// Asset ID to verify
        asset_id: String,
        
        /// Verify with specific public key
        #[arg(short, long)]
        public_key: Option<String>,
    },
    
    /// Sync with network
    Sync {
        /// Sync specific peer
        #[arg(short, long)]
        peer: Option<String>,
        
        /// Force full sync
        #[arg(long)]
        force: bool,
    },
    
    /// Node status and information
    Status {
        /// Show detailed status
        #[arg(short, long)]
        detailed: bool,
    },
    
    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    
    /// Bridge to external networks (Sprint 10.1)
    Bridge {
        #[command(subcommand)]
        action: BridgeAction,
    },
    
    /// Identity and sovereignty management
    Identity {
        #[command(subcommand)]
        action: IdentityAction,
    },
}

#[derive(Subcommand)]
pub enum AssetAction {
    /// Register a new asset
    Register {
        /// Asset file path
        path: String,
        
        /// Asset type (academic, certification, audio, video)
        #[arg(long)]
        asset_type: String,
    },
    
    /// List local assets
    List {
        /// Maximum number of assets to show
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    
    /// Verify an asset
    Verify {
        /// Asset ID to verify
        id: String,
    },
    
    /// Create encrypted backup of all assets
    Backup {
        /// Backup password (min 12 characters)
        #[arg(long)]
        password: Option<String>,
        
        /// Output file path (optional)
        #[arg(long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Show current configuration
    Show,
    
    /// Set configuration value
    Set {
        /// Configuration key
        key: String,
        /// Configuration value
        value: String,
    },
    
    /// Reset configuration to defaults
    Reset {
        /// Configuration key to reset (optional)
        key: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum BridgeAction {
    /// Encode for EVM networks (Ethereum, Arbitrum, etc.)
    Evm {
        /// Asset ID to bridge
        asset_id: String,
        
        /// Target network (arbitrum_mainnet, ethereum_mainnet, etc.)
        #[arg(long, default_value = "arbitrum_mainnet")]
        network: String,
        
        /// Dry run mode (don't execute transaction)
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Sign EVM transaction with secp256k1 key
    EvmSign {
        /// Calldata to sign
        calldata: String,
    },
    
    /// Serialize for Solana networks
    Solana {
        /// Asset ID to bridge
        asset_id: String,
        
        /// Target program ID
        #[arg(long)]
        program: String,
    },
}

#[derive(Subcommand)]
pub enum IdentityAction {
    /// Initialize node identity with BIP-39 seed
    Init {
        /// Force initialization even if identity already exists
        #[arg(short, long)]
        force: bool,
    },
    
    /// Show current identity information
    Show,
    
    /// Generate invitation package for lineage extension
    Invite,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize tracing
    let log_level = if cli.debug { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(log_level)))
        .init();
    
    // Validate network
    let network = match cli.network.as_str() {
        "devnet" | "testnet" | "mainnet" => cli.network,
        _ => return Err(anyhow::anyhow!("Invalid network mode. Use: devnet, testnet, or mainnet")),
    };
    
    tracing::info!("PIVA CLI starting on {} network", network);
    tracing::info!("JSON output: {}", cli.json);
    
    match cli.command {
        Commands::Init { force } => {
            handle_init(force, cli.json).await
        }
        Commands::Serve { port, discovery } => {
            tracing::info!("Starting PIVA node on port {}", port);
            tracing::info!("Discovery: {}", if discovery { "enabled" } else { "disabled" });
            tracing::warn!("🚧 Node serving not yet implemented");
            Ok(())
        }
        Commands::Asset { action } => {
            match action {
                AssetAction::Register { path, asset_type } => {
                    tracing::info!("Registering asset: {} ({})", path, asset_type);
                    handle_register(path, asset_type, cli.json).await
                }
                AssetAction::List { limit } => {
                    tracing::info!("Listing up to {} assets", limit);
                    handle_list(limit, cli.json).await
                }
                AssetAction::Verify { id } => {
                    tracing::info!("Verifying asset: {}", id);
                    handle_verify(id, cli.json).await
                }
                AssetAction::Backup { password, output } => {
                    tracing::info!("Creating encrypted backup");
                    handle_backup(password, output, cli.json).await
                }
            }
        }
        Commands::Verify { asset_id, public_key } => {
            tracing::info!("Verifying asset: {}", asset_id);
            if let Some(key) = &public_key {
                tracing::info!("Using public key: {}", key);
            }
            tracing::warn!("🚧 Standalone verify not yet implemented - use asset verify instead");
            Ok(())
        }
        Commands::Sync { peer, force } => {
            tracing::info!("Syncing with {} network", network);
            if let Some(p) = &peer {
                tracing::info!("Target peer: {}", p);
            }
            tracing::info!("Force sync: {}", force);
            tracing::warn!("🚧 Network sync not yet implemented");
            Ok(())
        }
        Commands::Status { detailed } => {
            handle_status(detailed, cli.json).await
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Show => {
                    handle_config("show", None, None, cli.json).await
                }
                ConfigAction::Set { key, value } => {
                    handle_config("set", Some(key), Some(value), cli.json).await
                }
                ConfigAction::Reset { key } => {
                    tracing::info!("Resetting config: {:?}", key);
                    tracing::warn!("🚧 Config reset not yet implemented");
                    Ok(())
                }
            }
        }
        Commands::Bridge { action } => {
            match action {
                BridgeAction::Evm { asset_id, network, dry_run } => {
                    tracing::info!("EVM bridge for asset {} to {}", asset_id, network);
                    handle_evm_encode(asset_id, dry_run, cli.json).await
                }
                BridgeAction::EvmSign { calldata } => {
                    tracing::info!("EVM signing for calldata");
                    handle_evm_sign(calldata, cli.json).await
                }
                BridgeAction::Solana { asset_id, program } => {
                    tracing::info!("Solana bridge for asset {} to program {}", asset_id, program);
                    tracing::warn!("🚧 Solana bridge not yet implemented - Sprint 11");
                    Ok(())
                }
            }
        }
        Commands::Identity { action } => {
            match action {
                IdentityAction::Init { force } => {
                    handle_identity_init(force, cli.json).await
                }
                IdentityAction::Show => {
                    handle_identity_show(cli.json).await
                }
                IdentityAction::Invite => {
                    handle_identity_invite(cli.json).await
                }
            }
        }
    }
}
