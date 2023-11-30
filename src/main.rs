use anyhow::Result;
use cfmms::dex::DexVariant;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::types::{BlockNumber, H160, U256};
use log::info;
use std::{str::FromStr, sync::Arc};

use evm_simulation::arbitrage::{simulate_triangular_arbitrage, TriangularArbitrage};
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
    let ws = Ws::connect(&env.wss_url).await.unwrap();
    let provider = Arc::new(Provider::new(ws));

    let block = provider
        .get_block(BlockNumber::Latest)
        .await
        .unwrap()
        .unwrap();

    // Example of how you detect the erc20 contract creation
    // First, get the tx data from mempool or established block
    // let tx = provider
        // .get_transaction(H256::from_str("0xffd087d96dd52b0f6c399233bae1c1a842459a53a0a5883ebf6054f06f6110b3").unwrap())
        // .await
        // .unwrap()
        // .unwrap();
    
    // Second, call is_erc20_contract_creation with the input data of tx
    // if is_erc20_contract_creation(tx.input) {
        // If it returns true, retrieve the contract address from address and nounce
        // calculate_contract_address(tx.from, tx.nonce));

        // And, record in a list of a property in struct or write out in a file
    // }

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

    let usdt = H160::from_str("0xdAC17F958D2ee523a2206206994597C13D831ec7").unwrap();
    let arb_paths = generate_triangular_paths(&verified_pools, usdt);

    let owner = H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap();
    let amount_in = U256::from(10)
        .checked_mul(U256::from(10).pow(U256::from(6)))
        .unwrap();
    let balance_slot = honeypot_filter.balance_slots.get(&usdt).unwrap();
    let target_token = honeypot_filter.safe_token_info.get(&usdt).unwrap();
    for path in &arb_paths {
        let arb = TriangularArbitrage {
            amount_in,
            path: path.clone(),
            balance_slot: *balance_slot,
            target_token: target_token.clone(),
        };
        match simulate_triangular_arbitrage(
            arb,
            provider.clone(),
            owner,
            block.number.unwrap(),
            None,
        ) {
            Ok(_profit) => {}
            Err(_e) => {}
        }
    }

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
