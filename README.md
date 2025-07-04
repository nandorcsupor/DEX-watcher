**🔍 DEX-Watcher**

Real-time SOL/USDC price monitoring across Solana's top 3 DEXs
**🎯 Features**

📊 Multi-DEX Monitoring - Tracks prices from Raydium, Orca, and Meteora simultaneously
⛓️ On-Chain Data - Reads directly from blockchain using native Rust crates (no APIs)
🚀 Concurrent Architecture - Async Rust with Tokio for high-performance monitoring
📈 Real-Time Updates - Live price feeds with percentage changes and liquidity reserves
🔄 Auto-Reconnection - Resilient monitoring with automatic error recovery

**🏗️ Architecture**

Raydium - Classic AMM implementation
Orca Whirlpool - Concentrated liquidity pools
Meteora DLMM - Dynamic bin-based liquidity

**🛠️ Tech Stack**

Rust - Zero-overhead concurrency without GIL limitations
Tokio - Async runtime for concurrent task execution
Solana Web3 - Direct blockchain interaction
Native Crates - Protocol-specific data parsing

**🚀 Usage**

`cargo run`

Output:
📊 SOL/USDC from Raydium: $143.2847 (+2.34%) (Reserves: 1247 SOL / 178432 USDC)
📊 SOL/USDC from Orca: $143.3102 (+2.41%) (Reserves: 892 SOL / 127651 USDC)
📊 SOL/USDC from Meteora: $143.2956 (+2.37%) (Reserves: 634 SOL / 90876 USDC)

`Perfect for arbitrage opportunities, market analysis, and DeFi research! 📈`
