use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::rpc_client::RpcClient
};
use solana_sdk::{account::Account, pubkey::Pubkey, commitment_config::CommitmentConfig};
use std::str::FromStr;
use tokio::sync::broadcast;
use std::sync::Arc;
use carbon_raydium_amm_v4_decoder::accounts::amm_info::AmmInfo as RaydiumAmmInfo;
use carbon_core::deserialize::CarbonDeserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmmInfo {
    pub pool_id: String,
    pub base_mint: String,    // SOL mint
    pub quote_mint: String,   // USDC mint  
    pub base_reserve: u64,    // SOL amount in pool
    pub quote_reserve: u64,   // USDC amount in pool
    pub price: f64,           // Calculated price
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub symbol: String,
    pub price: f64,
    pub change_percent: f64,
    pub timestamp: u64,
    pub source: String,
    pub base_reserve: u64,
    pub quote_reserve: u64,
}

pub struct RaydiumMonitor {
    rpc_client: RpcClient,
    sol_usdc_pool: Pubkey,
    price_cache: Option<f64>,
}

impl RaydiumMonitor {
    pub fn new() -> Self {
        // Solana Mainnet RPC (free)
        let rpc_client = RpcClient::new_with_commitment(
            "https://api.mainnet-beta.solana.com".to_string(),
            CommitmentConfig::confirmed(),
        );
        
        let sol_usdc_pool = Pubkey::from_str("58oQChx4yWmvKdwLLZzBi4ChoCc2fqCUWBkwMihLYQo2")
            .expect("Invalid pool address");
            
        Self {
            rpc_client,
            sol_usdc_pool,
            price_cache: None,
        }
    }
    
    // Start monitoring the pool account for changes
    pub async fn start_monitoring(
        &mut self, 
        tx: Arc<broadcast::Sender<PriceUpdate>>
    ) -> anyhow::Result<()> {
        println!("🚀 Starting Raydium SOL/USDC pool monitoring...");
        
        loop {
            match self.fetch_pool_data().await {
                Ok(amm_info) => {
                    // Calculate price from reserves
                    let current_price = self.calculate_price(&amm_info);
                    
                    let price_update = PriceUpdate {
                        symbol: "SOL/USDC".to_string(),
                        price: current_price,
                        change_percent: self.calculate_change_percent(current_price),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs(),
                        source: "Raydium".to_string(),
                        base_reserve: amm_info.base_reserve,
                        quote_reserve: amm_info.quote_reserve,
                    };
                    
                    let _ = tx.send(price_update);
                    self.price_cache = Some(current_price);
                }
                Err(e) => {
                    eprintln!("❌ Raydium fetch error: {}", e);
                }
            }
            
            // Poll every 2 seconds (much faster than API polling)
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    }
    
    // Fetch pool account data from Solana blockchain
    async fn fetch_pool_data(&self) -> anyhow::Result<AmmInfo> {
        // Get the pool account data directly from blockchain
        let account = self.rpc_client
            .get_account_with_commitment(&self.sol_usdc_pool, CommitmentConfig::confirmed())
            .await?
            .value
            .ok_or_else(|| anyhow::anyhow!("Pool account not found"))?;
            
        // Parse the account data (this is where AMM-specific parsing happens)
        self.parse_raydium_pool_data(&account).await
    }

    async fn get_token_account_balance(&self, token_account: &Pubkey) -> anyhow::Result<u64> {
        let balance = self.rpc_client
            .get_token_account_balance(token_account)
            .await?;
        
        Ok(balance.amount.parse::<u64>()?)
    }
    
    // Parse raw Raydium account data into structured info
    async fn parse_raydium_pool_data(&self, account: &Account) -> anyhow::Result<AmmInfo> {
        let data = &account.data;
        
        if data.len() < 656 {  
            return Err(anyhow::anyhow!("Invalid pool account data size"));
        }
        
        // 🔥 PROPER PARSING WITH CARBON DECODER! 🔥
        match <RaydiumAmmInfo as CarbonDeserialize>::deserialize(data) {
            Some(raydium_info) => {
                // Get the actual reserves from token vault accounts
                let base_vault_amount = self.get_token_account_balance(&raydium_info.token_coin).await?;
                let quote_vault_amount = self.get_token_account_balance(&raydium_info.token_pc).await?;
                
                Ok(AmmInfo {
                    pool_id: self.sol_usdc_pool.to_string(),
                    base_mint: raydium_info.coin_mint.to_string(),
                    quote_mint: raydium_info.pc_mint.to_string(),
                    base_reserve: base_vault_amount,
                    quote_reserve: quote_vault_amount,
                    price: 0.0,
                })
            }
            None => {
                eprintln!("❌ Failed to parse Raydium data");
                println!("🔍 Raw data length: {} bytes", data.len());
                println!("🔍 First 64 bytes: {}", hex::encode(&data[..64.min(data.len())]));
                
                Err(anyhow::anyhow!("Failed to parse Raydium AMM data"))
            }
        }
    }
    
    // Calculate price from AMM reserves (x * y = k formula)
    fn calculate_price(&self, amm_info: &AmmInfo) -> f64 {
        if amm_info.base_reserve == 0 {
            return 0.0;
        }
        
        // Price = quote_reserve / base_reserve (adjusted for decimals)
        // SOL has 9 decimals, USDC has 6 decimals
        let sol_amount = amm_info.base_reserve as f64 / 1e9;    // Convert lamports to SOL
        let usdc_amount = amm_info.quote_reserve as f64 / 1e6;  // Convert micro-USDC to USDC
        
        return usdc_amount / sol_amount;  // Price of SOL in USDC
    }
    
    // Calculate percentage change from cached price
    fn calculate_change_percent(&self, current_price: f64) -> f64 {
        match self.price_cache {
            Some(cached_price) => {
                ((current_price - cached_price) / cached_price) * 100.0
            }
            None => 0.0,
        }
    }
}