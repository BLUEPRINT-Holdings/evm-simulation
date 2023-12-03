use alloy_primitives::address;
use anyhow::Result;
use cfmms::dex::DexVariant;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{BlockNumber, H160, U256};
use ethers_core::types::BlockId;
use evm_simulation::trace::EvmTracer;
use log::info;
use std::{str::FromStr, sync::Arc};
use url::Url;

// use evm_simulation::arbitrage::{simulate_triangular_arbitrage, TriangularArbitrage};
use evm_simulation::constants::Env;
use evm_simulation::honeypot::HoneypotFilter;
use evm_simulation::paths::generate_triangular_paths;
use evm_simulation::pools::{load_all_pools, Pool};

use evm_simulation::utils::setup_logger;

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

    let block = provider
        .get_block(BlockNumber::Latest)
        .await
        .unwrap()
        .unwrap();

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

    let token_contract = address!("8c66560b19505e6aE79F09ffb1DBBb70F067E39d");
    // NOTE: the usage of check_admin function below
    // If the contract doesn't have admin address at this moment, then the function returns (false, zero_address)
    // If exists, it returns (true, admin_address)
    let admin = honeypot_filter.simulator.check_admin(token_contract).unwrap();
    println!("admin res: {:?}", admin);

    let block_number = &honeypot_filter.simulator.block_number;
    let traceer = EvmTracer::new(provider.to_owned());
    let nonce = provider
        .get_transaction_count(
            admin.1,
            Some(BlockId::Number(BlockNumber::Number(honeypot_filter.simulator.block_number))),
        )
        .await
        .unwrap();
    let evil_impl = traceer
        .check_possible_evil_implementation(
            H160::from_str("8c66560b19505e6aE79F09ffb1DBBb70F067E39d").unwrap(),
            admin.1,
            honeypot_filter.simulator.owner,
            nonce, 
            env.chain_id.into(),
            honeypot_filter.simulator.block_number.as_u64(),
        ).await;

        println!("eviil impl res: {:?}", evil_impl);

    // TODO: change the arg to &Vec<H160> to accept token contract address directly
    honeypot_filter.validate_token(&pools).await;

    /// NOTE: filter_tokens is now deprecated 
    /// we validate by each token contract, not pool contract anymore
    honeypot_filter
        .filter_tokens(&pools[0..5000].to_vec())
        .await;

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

    // let usdt = H160::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap();
    // let arb_paths = generate_triangular_paths(&verified_pools, usdt);
// 
    // let owner = H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap();
    // let amount_in = U256::from(10)
        // .checked_mul(U256::from(10).pow(U256::from(6)))
        // .unwrap();
    // let balance_slot = honeypot_filter.balance_slots.get(&usdt).unwrap();
    // let target_token = honeypot_filter.safe_token_info.get(&usdt).unwrap();
    // for path in &arb_paths {
        // let arb = TriangularArbitrage {
            // amount_in,
            // path: path.clone(),
            // balance_slot: *balance_slot,
            // target_token: target_token.clone(),
        // };
        // match simulate_triangular_arbitrage(
            // arb,
            // provider.clone(),
            // owner,
            // block.number.unwrap(),
            // None,
        // ) {
            // Ok(_profit) => {}
            // Err(_e) => {}
        // }
    // }

    // let (event_sender, _): (Sender<Event>, _) = broadcast::channel(512);

    // let mut set = JoinSet::new();

    // set.spawn(stream_new_blocks(provider.clone(), event_sender.clone()));
    // set.spawn(stream_pending_transactions(
    //     provider.clone(),
    //     event_sender.clone(),
    // ));
    // set.spawn(event_handler(provider.clone(), event_sender.clone()));

    // while let Some(res) = set.join_next().await {
    //     info!("{:?}", res);
    // }

    Ok(())
}
