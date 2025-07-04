use carbon_meteora_dlmm_decoder::accounts::lb_pair::LbPair;
use carbon_core::deserialize::CarbonDeserialize;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{ pubkey::Pubkey};
use std::str::FromStr;
use tokio::sync::broadcast;
use anyhow::Result;
use std::sync::Arc;

use crate::raydium::PriceUpdate;

pub struct MeteoraMonitor {
   rpc_client: RpcClient,
   dlmm_pool_address: Pubkey,
   price_cache: Option<f64>,
}

impl MeteoraMonitor {
   pub fn new() -> Self {
       let rpc_client = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());
       
       let dlmm_pool_address = Pubkey::from_str("5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6").unwrap();
       
       Self {
           rpc_client,
           dlmm_pool_address,
           price_cache: None,
       }
   }

   pub async fn start_monitoring(&mut self, tx: Arc<broadcast::Sender<PriceUpdate>>) -> Result<()> {
       let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));
       
       loop {
           interval.tick().await;
           
           match self.fetch_dlmm_data().await {
               Ok((current_price, base_reserve, quote_reserve)) => {
                   let change_percent = if let Some(cached) = self.price_cache {
                       ((current_price - cached) / cached) * 100.0
                   } else {
                       0.0
                   };
                   
                   let update = PriceUpdate {
                       symbol: "SOL/USDC".to_string(),
                       source: "Meteora".to_string(),
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
                       println!("No receivers for Meteora price updates");
                   }
                   
                   self.price_cache = Some(current_price);
               }
               Err(e) => {
                   eprintln!("Failed to fetch Meteora DLMM price: {}", e);
               }
           }
       }
   }

   async fn fetch_dlmm_data(&self) -> Result<(f64, u64, u64)> {
       // Get DLMM pool account data
       let account = self.rpc_client.get_account(&self.dlmm_pool_address)?;
       
       // Parse account data with Carbon decoder (same pattern as Raydium)
       let data = &account.data;
       
       if data.len() < 100 {  
           return Err(anyhow::anyhow!("Invalid DLMM account data size"));
       }
       
       // 🔥 PROPER PARSING WITH CARBON DECODER! 🔥
       match <LbPair as CarbonDeserialize>::deserialize(data) {
           Some(lb_pair) => {
               // Get actual token vault balances
               let base_reserve = self.get_token_account_balance(&lb_pair.reserve_x).await?;
               let quote_reserve = self.get_token_account_balance(&lb_pair.reserve_y).await?;

               // Get active bin price
               let price = self.calculate_price_from_active_bin(
                    lb_pair.active_id,
                    lb_pair.bin_step
                );
               
               Ok((price, base_reserve, quote_reserve))
           }
           None => {
               eprintln!("❌ Failed to parse Meteora DLMM data");
               println!("🔍 Raw data length: {} bytes", data.len());
               println!("🔍 First 64 bytes: {}", hex::encode(&data[..64.min(data.len())]));
               
               Err(anyhow::anyhow!("Failed to parse Meteora DLMM data"))
           }
       }
   }

   // METEORA DLMM PRICE FORMULA:
    // ===========================
    // active_bin_price = base_price × (1 + bin_step/10000)^active_id
    //
    // Where:
    // - base_price = pool's "center" reference price
    // - bin_step = percentage difference between bins (e.g. 10 = 0.1%)
    // - active_id = current active bin identifier (negative = higher price)
    //
    // Example: active_id = -1963, bin_step = 10
    // → Current price is 1963 bins HIGHER than base_price
    // → Each bin differs by 0.1% → ~19.6% total deviation

   fn calculate_price_from_active_bin(&self, active_id: i32, bin_step: u16) -> f64 {
        // Meteora DLMM exact formula: price = (1 + bin_step/10000)^active_id
        let bin_step_decimal = bin_step as f64 / 10000.0;
        let base_multiplier = 1.0 + bin_step_decimal;
        
        // Calculate the actual price using the DLMM formula
        let active_bin_price = base_multiplier.powi(active_id);

        let final_price = active_bin_price * 1000.0;
        return final_price;
    }

   async fn get_token_account_balance(&self, token_account: &Pubkey) -> anyhow::Result<u64> {
       let balance = self.rpc_client
           .get_token_account_balance(token_account)?;
       
       Ok(balance.amount.parse::<u64>()?)
   }
}