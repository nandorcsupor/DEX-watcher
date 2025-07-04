use orca_whirlpools_client::Whirlpool;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{account_info::AccountInfo, pubkey::Pubkey};
use std::str::FromStr;
use tokio::sync::broadcast;
use anyhow::Result;
use std::sync::Arc;

use crate::raydium::PriceUpdate;

pub struct OrcaMonitor {
    rpc_client: RpcClient,
    whirlpool_address: Pubkey,
    price_cache: Option<f64>,
}

impl OrcaMonitor {
    pub fn new() -> Self {
        let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
        
        let whirlpool_address = Pubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap();
        
        Self {
            rpc_client,
            whirlpool_address,
            price_cache: None,
        }
    }

    pub async fn start_monitoring(&mut self, tx: Arc<broadcast::Sender<PriceUpdate>>) -> Result<()> {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
        
        loop {
            interval.tick().await;
            
            match self.fetch_whirlpool_data().await {
                Ok((current_price, base_reserve, quote_reserve)) => {
                    let change_percent = if let Some(cached) = self.price_cache {
                        ((current_price - cached) / cached) * 100.0
                    } else {
                        0.0
                    };
                    
                    let update = PriceUpdate {
                        symbol: "SOL/USDC".to_string(),
                        source: "Orca".to_string(),
                        price: current_price,
                        change_percent,
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        base_reserve,
                        quote_reserve,
                    };
                    
                    if tx.send(update).is_err() {
                        println!("No receivers for Orca price updates");
                    }
                    
                    self.price_cache = Some(current_price);
                }
                Err(e) => {
                    eprintln!("Failed to fetch Orca price: {}", e);
                }
            }
        }
    }

    async fn fetch_whirlpool_data(&self) -> Result<(f64, u64, u64)> {
    let account = self.rpc_client.get_account(&self.whirlpool_address)?;
    
    let mut lamports = account.lamports;
    let mut data = account.data;

    let account_info = AccountInfo::new(
        &self.whirlpool_address,
        false,
        false, 
        &mut lamports,
        &mut data,
        &account.owner,
        false,
        account.rent_epoch,
    );
    
    let whirlpool = Whirlpool::try_from(&account_info)?;

    let base_balance = self.rpc_client.get_token_account_balance(&whirlpool.token_vault_a)?;
    let quote_balance = self.rpc_client.get_token_account_balance(&whirlpool.token_vault_b)?;
    
    let base_reserve = base_balance.amount.parse::<u64>()?;
    let quote_reserve = quote_balance.amount.parse::<u64>()?;
    
    let price = whirlpool_price_from_sqrt_price(
        whirlpool.sqrt_price,
        6, // SOL decimals  
        9, // USDC decimals
    );
    
    Ok((price, base_reserve, quote_reserve))
    }
}

// Orca Whirlpool sqrt_price -> price conversion
fn whirlpool_price_from_sqrt_price(sqrt_price: u128, token_a_decimals: u8, token_b_decimals: u8) -> f64 {
    // Orca formula: price = (sqrt_price / 2^64)^2 * 10^(decimals_b - decimals_a)
    let sqrt_price_f64 = sqrt_price as f64;
    let q64 = (1u128 << 64) as f64;
    let price_raw = (sqrt_price_f64 / q64).powi(2);
    let decimal_adjustment = 10_f64.powi(token_b_decimals as i32 - token_a_decimals as i32);
    
    price_raw * decimal_adjustment
}