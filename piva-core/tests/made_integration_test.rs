//! # MADE Integration Tests
//!
//! Tests for the autonomous decision engine and tick loop.

use piva_core::{MadeAgent, MadeConfig, ResourceMetrics, MadeDecision, made::now_secs};
use std::thread;
use std::time::Duration;

#[test]
fn test_made_tick_loop_basic() -> Result<(), Box<dyn std::error::Error>> {
    // Create configuration with fast tick for testing
    let config = MadeConfig {
        tick_interval_secs: 1, // 1 second for quick testing
        decision_probability: 1.0, // Always make decisions for testing
        ..MadeConfig::default()
    };
    
    let agent = MadeAgent::new(config);
    
    // Verify initial state
    assert!(!agent.is_running()?);
    
    // Start the agent
    agent.start()?;
    assert!(agent.is_running()?);
    
    // CAMBIO: Polling dinámico en lugar de sleep fijo
    let mut decision_found = false;
    for attempt in 0..5 { // Reintentar por 5 segundos máximo
        if agent.get_decision_history()?.len() > 0 {
            decision_found = true;
            println!("🎯 Decision found after {} attempts", attempt + 1);
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }

    assert!(decision_found, "MADE no generó decisiones en el tiempo esperado");
    
    let metrics = agent.get_metrics()?;
    assert!(metrics.timestamp > 0, "Métricas estancadas");
    
    let history = agent.get_decision_history()?;
    println!("📊 Made {} decisions in {} seconds", history.len(), 5);
    
    // Print some decisions for debugging
    for (timestamp, decision) in history.iter().take(3) {
        println!("🤖 Decision at {}: {:?}", timestamp, decision);
    }
    
    // Stop the agent
    agent.stop()?;
    assert!(!agent.is_running()?);
    
    println!("✅ MADE tick loop test completed successfully");
    
    Ok(())
}

#[test]
fn test_made_resource_pressure_decision() -> Result<(), Box<dyn std::error::Error>> {
    // Test with high resource pressure
    let config = MadeConfig {
        ram_threshold: 0.5, // Lower threshold
        disk_threshold: 0.5,
        decision_probability: 1.0,
        ..MadeConfig::default()
    };
    
    let agent = MadeAgent::new(config);
    
    // Simulate high resource pressure using default for better isolation
    let high_pressure_metrics = ResourceMetrics {
        ram_usage_percent: 0.9, // Above threshold
        disk_usage_percent: 0.8, // Above threshold
        timestamp: now_secs(),
        ..ResourceMetrics::default()
    };
    
    // Test decision making directly
    let decision = agent.make_decision_direct(&high_pressure_metrics);
    
    // Should suggest pruning due to resource pressure
    match decision {
        MadeDecision::PruneAssets { count, reason } => {
            assert!(count > 0, "Should suggest pruning some assets");
            assert!(reason.contains("Resource pressure"), "Reason should mention resource pressure");
            println!("✅ Correctly suggested pruning: {} assets - {}", count, reason);
        }
        other => panic!("Expected PruneAssets decision, got {:?}", other),
    }
    
    Ok(())
}

#[test]
fn test_made_no_connections_decision() -> Result<(), Box<dyn std::error::Error>> {
    let config = MadeConfig {
        decision_probability: 1.0,
        ..MadeConfig::default()
    };
    
    let agent = MadeAgent::new(config);
    
    // Simulate no connections using default for better isolation
    let no_connections_metrics = ResourceMetrics {
        active_connections: 0, // No connections
        timestamp: now_secs(),
        ..ResourceMetrics::default()
    };
    
    let decision = agent.make_decision_direct(&no_connections_metrics);
    
    // Should suggest seeking peers
    match decision {
        MadeDecision::SeekPeers { min_score, max_attempts } => {
            assert!(min_score >= 0, "Should have reasonable minimum score");
            assert!(max_attempts > 0, "Should have positive max attempts");
            println!("✅ Correctly suggested peer seeking: score >= {}, attempts = {}", min_score, max_attempts);
        }
        other => panic!("Expected SeekPeers decision, got {:?}", other),
    }
    
    Ok(())
}

#[test]
fn test_made_cpu_pressure_decision() -> Result<(), Box<dyn std::error::Error>> {
    let initial_interval_secs = 10;
    let config = MadeConfig {
        cpu_threshold: 0.5, // Lower threshold
        decision_probability: 1.0,
        tick_interval_secs: initial_interval_secs,
        ram_threshold: 0.95, // Very high to avoid RAM pressure
        disk_threshold: 0.95, // Very high to avoid disk pressure
        min_peer_score: 1000, // Very high to avoid peer seeking
        ..MadeConfig::default()
    };
    
    let agent = MadeAgent::new(config);
    
    // Simulate high CPU pressure with NO other conditions
    let high_cpu_metrics = ResourceMetrics {
        cpu_usage_percent: 0.9, // 90% CPU - above threshold
        ram_usage_percent: 0.3, // Below RAM threshold
        disk_usage_percent: 0.3, // Below disk threshold
        active_connections: 5, // Has connections to avoid peer seeking
        timestamp: now_secs(),
        ..ResourceMetrics::default()
    };
    
    let decision = agent.make_decision_direct(&high_cpu_metrics);
    
    // Should suggest performance adjustment
    match decision {
        MadeDecision::AdjustPerformance { new_interval_ms, reason } => {
            // Validación relativa al intervalo inicial
            let initial_ms = initial_interval_secs * 1000;
            assert!(new_interval_ms > initial_ms, 
                "El nuevo intervalo ({}) debe ser mayor al inicial ({})", 
                new_interval_ms, initial_ms);
            assert!(reason.contains("CPU pressure"), 
                "Reason should mention CPU pressure, got: {}", reason);
            println!("✅ Correctly suggested performance adjustment: {}ms - {}", new_interval_ms, reason);
        }
        other => panic!("Expected AdjustPerformance decision due to CPU pressure, got {:?}", other),
    }
    
    Ok(())
}
