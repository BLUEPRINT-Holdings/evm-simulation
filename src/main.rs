use anyhow::{anyhow, Result};
use cfmms::dex::DexVariant;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{Block, BlockNumber, H160, H256, U256};
use ethers_providers::Http;
use evm_simulation::gmx::{fetch_token_price, GmxPlayground};
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

    // let provider = Arc::new(Provider::new(ws));
    // let http_url = Url::parse(&env.https_url).expect("Failed to parse HTTP URL");

    let provider = Arc::new(Provider::<Http>::try_from(&env.https_url).unwrap());

    let block = provider.get_block(BlockNumber::Latest).await.unwrap().unwrap();
    let args: Vec<String> = env::args().collect();

    // args[0] はプログラム自体のパスです。args[1] が最初の引数
    if args.len() > 1 {
        match args[1].as_str() {
            "gmxv2" => {
                println!("GMX V2 test started.");
                gmx_v2_test(provider, block.clone()).await;
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

async fn gmx_v2_test(provider: Arc<Provider<Http>>, block: Block<H256>) {
    let prices = fetch_token_price("ETH".to_string()).await;
    println!("Prices: {:?}", prices.unwrap());
    let mut gmx_playground = GmxPlayground::new(provider.clone(), block.clone());
    let eth_amount = 1;
    // set eth balance to owner address
    gmx_playground.simulator.set_eth_balance(eth_amount);
    let eth_balance = gmx_playground.simulator.get_eth_balance();
    println!("ETH Balance: {:?}", eth_balance);

    // let usdc = H160::from_str("0xaf88d065e77c8cC2239327C5EDb3A432268e5831").unwrap();
    // gmx_playground.simulator.set_token_balance(gmx_playground.simulator.owner,usdc, 18, U256::from(1000000000));

    // in case of using eth for deposit, dont need to approve before
    // directly defining weth token address for now
    let collateral_token = "ETH";
    let collateral_amount = 0.5;
    let size_delta_usd = 1000_f64;
    let create_position_res = gmx_playground
        .create_short_position(collateral_token, collateral_amount, size_delta_usd)
        .await;
    // let res = gmx_playground.simulator.provider.call(tx, None).await;
    println!("Create Position res: {:?}", create_position_res);
    // let market_token = H160::from_str("0x70d95587d40A2caf56bd97485aB3Eec10Bee6336").unwrap();

    // See ETH is consumed for deposit
    let eth_balance = gmx_playground.simulator.get_eth_balance();
    println!("ETH Balance: {:?}", eth_balance);

    let positions = gmx_playground.get_account_positions();
    // let position_info = gmx_playground.get_position_info(market_token);
    // println!("Position Info: {:?}", position_info);
}

async fn honeypot_test(env: Env, provider: Arc<Provider<Http>>, block: Block<H256>) {
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
