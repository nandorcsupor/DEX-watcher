mod raydium;
mod orca;
mod meteora;

use raydium::RaydiumMonitor;
use orca::OrcaMonitor;
use meteora::MeteoraMonitor;

use std::sync::Arc;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    env_logger::init();
    
    // Create broadcast channel for price updates from all AMMs
    let (tx, mut rx) = broadcast::channel(1000);
    let tx = Arc::new(tx);
    
    // Start all AMM monitors concurrently with join handles
    let raydium_handle = {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut raydium = RaydiumMonitor::new();
            loop {
                match raydium.start_monitoring(tx.clone()).await {
                    Ok(_) => {
                        println!("✅ Raydium monitoring ended normally");
                    }
                    Err(e) => {
                        eprintln!("❌ Raydium error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        println!("🔄 Reconnecting to Raydium...");
                    }
                }
            }
        })
    };
    
    // Start Orca Whirlpool monitoring 
    let orca_handle = {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut orca = OrcaMonitor::new();
            loop {
                match orca.start_monitoring(tx.clone()).await {
                    Ok(_) => {
                        println!("✅ Orca monitoring ended normally");
                    }
                    Err(e) => {
                        eprintln!("❌ Orca error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        println!("🔄 Reconnecting to Orca...");
                    }
                }
            }
        })
    };

    // Start Meteora DLMM monitoring
    let meteora_handle = {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut meteora = MeteoraMonitor::new();
            loop {
                match meteora.start_monitoring(tx.clone()).await {
                    Ok(_) => {
                        println!("✅ Meteora monitoring ended normally");
                    }
                    Err(e) => {
                        eprintln!("❌ Meteora error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        println!("🔄 Reconnecting to Meteora...");
                    }
                }
            }
        })
    };
    
    // Demo: Print all price updates from any AMM
    let price_display_handle = tokio::spawn(async move {
        while let Ok(price_update) = rx.recv().await {
            println!("📊 {} from {}: ${:.4} ({:+.2}%) (Reserves: {} SOL / {} USDC)", 
                price_update.symbol,
                price_update.source, 
                price_update.price,
                price_update.change_percent,
                price_update.base_reserve as f64 / 1e9,
                price_update.quote_reserve as f64 / 1e6,
            );
        }
    });
    
    println!("🚀 AMM Price Monitor started! Monitoring:");
    println!("   - Raydium SOL/USDC (Classic AMM)");
    println!("   - Orca Whirlpool SOL/USDC (Concentrated Liquidity)");
    println!("   - Meteora DLMM SOL/USDC (Dynamic Bins)");
    println!("Press Ctrl+C to exit");
    
    // Wait for shutdown signal or any task to complete
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("🛑 Shutdown signal received...");
        }
        _ = raydium_handle => {
            println!("🛑 Raydium task ended");
        }
        _ = orca_handle => {
            println!("🛑 Orca task ended");
        }
        _ = meteora_handle => {
            println!("🛑 Meteora task ended");
        }
        _ = price_display_handle => {
            println!("🛑 Price display task ended");
        }
    }
    
    println!("🛑 Shutting down all monitors...");
}