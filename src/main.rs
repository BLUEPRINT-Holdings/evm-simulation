use anyhow::{Result, anyhow};
use cfmms::dex::DexVariant;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Block, BlockNumber, H160, H256, U256};
use evm_simulation::gmx::GmxPlayground;
use log::info;
use std::env;
use std::str::FromStr;
use std::sync::Arc;

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
    let args: Vec<String> = env::args().collect();

    // args[0] はプログラム自体のパスです。args[1] が最初の引数
    if args.len() > 1 {
        match args[1].as_str() {
            "gmxv2" => {
                tokio::spawn(async move {
                    println!("GMX V2 test started.");
                    gmx_v2_test(provider.clone(), block.clone()).await;
                });
            }
            "honeypot" => {
                tokio::spawn(async move {
                    honeypot_test(env, provider.clone(), block.clone()).await;
                });
            }
            _ => println!("Invalid input. Please use 'gmxv2' or 'honeypot'."),
        }
    } else {
        println!("No string was received.");
        return Err(anyhow!("No string was received."));
    }

    Ok(())
}

async fn gmx_v2_test(provider: Arc<Provider<Ws>>, block:  Block<H256>) {
    let mut gmx_playground = GmxPlayground::new(provider.clone(), block.clone());
    let eth_amount = 1;
    // set eth balance to owner address
    gmx_playground.simulator.set_eth_balance(eth_amount);
    // in case of using eth for deposit, dont need to approve before
    // directly defining weth token address for now
    let collateral_token = H160::from_str("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1").unwrap();
    let collateral_amount = U256::from(1000000000);
    let size_delta_usd = U256::from(1000000000);
    let create_position_res = gmx_playground.create_short_position(collateral_token, collateral_amount, size_delta_usd);
    println!("Create Position res: {:?}", create_position_res);

}

async fn honeypot_test(env: Env, provider: Arc<Provider<Ws>>, block:  Block<H256>) {
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
    let pools = load_all_pools(env.wss_url.clone(), factories).await.unwrap();

    let mut honeypot_filter = HoneypotFilter::new(provider.clone(), block.clone());
    honeypot_filter.setup().await;

    // Buy: 5%, Sell: 5%
    // let token_addr = H160::from_str("0x24EdDeD3f03abb2e9D047464294133378bddB596").unwrap();
    // let pool_addr = H160::from_str("0x15842C52c5A8730F028708e3492e1ab0Be59Bd80").unwrap();

    // honeypot_filter.validate_token_on_simulate_swap(token_addr, pool_addr, None, None).await;
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
}