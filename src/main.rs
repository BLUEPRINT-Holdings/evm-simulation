use anyhow::Result;
use cfmms::dex::DexVariant;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{BlockNumber, H160};
use log::info;
use std::{str::FromStr, sync::Arc};

use evm_simulation::constants::Env;
use evm_simulation::honeypot::HoneypotFilter;
use evm_simulation::pools::{load_all_pools, Pool};

use evm_simulation::utils::setup_logger;
use url::Url;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    setup_logger()?;

    info!("[⚡️🦀⚡️ Starting EVM simulation]");

    let env = Env::new();
    // let ws = Ws::connect(&env.wss_url).await.unwrap();
    let wss_url = format!("{}?key={}", &env.wss_url, &env.api_key);
    let wss_url = Url::parse(&wss_url).expect("Failed to parse WSS URL");
    let ws = Ws::connect(wss_url).await.unwrap();

    let provider = Arc::new(Provider::new(ws));

    let block = provider.get_block(BlockNumber::Latest).await.unwrap().unwrap();

    let factories = vec![
        (
            // Uniswap v2
            "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
            DexVariant::UniswapV2,
            10000835u64,
        ),
        (
            // Sushiswap V2
            "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac",
            DexVariant::UniswapV2,
            10794229u64,
        ),
    ];
    let pools = load_all_pools(env.wss_url.clone(), factories).await?;

    let mut honeypot_filter = HoneypotFilter::new(provider.clone(), block.clone());
    honeypot_filter.setup().await;

    // Buy: 5%, Sell: 5%
    let token_addr = H160::from_str("0x24EdDeD3f03abb2e9D047464294133378bddB596").unwrap();
    let pool_addr = H160::from_str("0x15842C52c5A8730F028708e3492e1ab0Be59Bd80").unwrap();

    honeypot_filter.validate_token_on_simulate_swap(token_addr, pool_addr, None, None).await;
    // honeypot_filter.filter_tokens(&pools[0..5000].to_vec()).await;

    let verified_pools: Vec<Pool> = pools
        .into_iter()
        .filter(|pool| {
            let token0_verified = honeypot_filter.safe_token_info.contains_key(&pool.token0)
                || honeypot_filter.token_info.contains_key(&pool.token0);
            let token1_verified = honeypot_filter.safe_token_info.contains_key(&pool.token1)
                || honeypot_filter.token_info.contains_key(&pool.token1);
            token0_verified && token1_verified
        })
        .collect();
    info!("Verified pools: {:?} pools", verified_pools.len());

    Ok(())
}
