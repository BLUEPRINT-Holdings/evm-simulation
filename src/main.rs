use anyhow::{anyhow, Result};
use cfmms::dex::DexVariant;
use csv::Reader;
use ethers::etherscan::Client;
use ethers::middleware::SignerMiddleware;
use ethers::providers::{Middleware, Provider, Ws};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::types::{
    Block, BlockNumber, Eip1559TransactionRequest, NameOrAddress, TransactionReceipt,
    TransactionRequest, H160, H256, U256,
};
use ethers_providers::{Http, PendingTransaction};
use evm_simulation::gmx::{expand_decimals, fetch_token_price, GmxPlayground, GmxV2Reader, DATA_STORE, EXCHANGE_ROUTER, READER, REFERRAL_STORAGE};
use evm_simulation::interfaces::gmx::{account_position_list_key, claimable_funding_amount_key, get_position_key, MarketPrices, PositionInfo, PriceProps, Token};
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
    let mut wallet = env.priv_key.parse::<LocalWallet>().unwrap();

    // let provider = Arc::new(Provider::new(ws));
    // let http_url = Url::parse(&env.https_url).expect("Failed to parse HTTP URL");

    let provider = Arc::new(Provider::<Http>::try_from(&env.https_url).unwrap());
    let chain_id = provider.get_chainid().await.unwrap();
    wallet = wallet.with_chain_id(chain_id.as_u64());
    let client = SignerMiddleware::new(provider.clone(), wallet.clone());

    let block = provider.get_block(BlockNumber::Latest).await.unwrap().unwrap();
    let args: Vec<String> = env::args().collect();

    // args[0] はプログラム自体のパスです。args[1] が最初の引数
    if args.len() > 1 {
        match args[1].as_str() {
            "gmxv2" => {
                println!("GMX V2 test started.");
                execute_on_main(provider, &client, block.clone()).await;
                // gmx_v2_on_simulation(provider.clone(), block).await;
                // send_simple_transaction(&client.clone()).await.unwrap();
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

pub async fn execute_on_main(
    provider: Arc<Provider<Http>>,
    client: &SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
    block: Block<H256>,
) {
    let mut gmx_playground = GmxPlayground::new(provider.clone(), block.clone());
    let reader = GmxV2Reader::new(*READER, provider.clone());

    let collateral_token = "ETH";
    let collateral_amount = 0.002;
    let size_delta_usd = 12_f64;
    let eth_token_decimals = Token::from_name("ETH").unwrap().info().decimals;
    // Set execution fee to 0.0002 ETH
    let exec_fee = expand_decimals(0.0002, eth_token_decimals);

    let typed_tx = gmx_playground
        .create_short_position_tx(collateral_token, collateral_amount, size_delta_usd, exec_fee)
        .await
        .unwrap();
    let typed_tx: TypedTransaction = TypedTransaction::Eip1559(typed_tx);
    // let signature = client.sign_transaction(&typed_tx, gmx_playground.simulator.owner).await.unwrap();
    // println!("Signed transaction: {:?}", signature);
    // let pending_tx: PendingTransaction<'_, Http> = client.send_transaction(typed_tx, None).await.unwrap();
    // send_transaction(&client, typed_tx).await.unwrap();

    /// Query
    // getAccountPositions
    // get calldata using gmx_v2_abi
    // let get_account_positions_calldata = gmx_playground.get_account_positions();
    // // call reader contract with calldata
    // let query_tx = gmx_playground.fill_tx_fields(get_account_positions_calldata).await.unwrap();
    // let query_tx: TypedTransaction = TypedTransaction::Eip1559(query_tx);
    // let res = reader
    //     .get_account_positions(*DATA_STORE, gmx_playground.simulator.owner, U256::zero(), U256::one())
    //     .call()
    //     .await
    //     .unwrap();
    // println!("Account positions: {:?}", res);

    // getPositionInfo
    // Call this for the latest funding amount and position info
    let calldata = gmx_playground.get_position_info_calldata(H160::from_str("0x70d95587d40A2caf56bd97485aB3Eec10Bee6336").unwrap()).await;
    let query_tx: TypedTransaction = TypedTransaction::Eip1559(gmx_playground.fill_tx_fields(calldata).await.unwrap());
    let res = reader.client().call(&query_tx, None).await.unwrap();
    let position_info: PositionInfo = reader.decode_output("getPositionInfo", res).unwrap();
    println!("Position Info: {:?}", position_info);

    let accrued_funding_in_usd = gmx_playground.get_accrued_funding_fee_in_usd(position_info).await;
    println!("Accrued funding in USD: {:?}", accrued_funding_in_usd);
}

// NOTE: In conclusion, functions on simulation weren't working as expected.
async fn gmx_v2_on_simulation(provider: Arc<Provider<Http>>, block: Block<H256>) {
    let mut gmx_playground = GmxPlayground::new(provider.clone(), block.clone());
    let eth_amount = 1;
    // set eth balance to owner address
    gmx_playground.simulator.set_eth_balance(eth_amount);
    // let eth_balance = gmx_playground.simulator.get_eth_balance();
    // let usdc = H160::from_str("0xaf88d065e77c8cC2239327C5EDb3A432268e5831").unwrap();
    // gmx_playground.simulator.set_token_balance(gmx_playground.simulator.owner,usdc, 18, U256::from(1000000000));

    // in case of using eth for deposit, dont need to approve before
    // directly defining weth token address for now
    // let collateral_token = "ETH";
    // let collateral_amount = 0.5;
    // let size_delta_usd = 1000_f64;
    // let create_position_res = gmx_playground
    // .create_short_position(collateral_token, collateral_amount, size_delta_usd)
    // .await;
    // let res = gmx_playground.simulator.provider.call(tx, None).await;
    // println!("Create Position res: {:?}", create_position_res);
    let market_token = H160::from_str("0x70d95587d40A2caf56bd97485aB3Eec10Bee6336").unwrap();

    // let positions = gmx_playground.get_account_positions();
    // println!("Positions: {:?}", positions);
    // let position_info = gmx_playground.get_position_info_calldata(market_token).await;
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

async fn send_simple_transaction(
    client: &SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = client.clone();
    let nonce = client.get_transaction_count(client.address(), None).await.unwrap();

    let to_address: H160 = "RECEIVER".parse()?;
    let value = U256::from(100000000000000u64); // 0.0001 ETH in wei
    let client = client.clone();
    let gas_price = client.provider().get_gas_price().await.unwrap();
    let client = client.clone();
    let chain_id = client.get_chainid().await.unwrap();
    println!("Chain ID: {:?}", chain_id);

    let tx_request = TransactionRequest {
        from: Some(client.address()),
        to: Some(to_address.into()),
        nonce: Some(nonce),
        gas: Some(U256::from(50000)), 
        gas_price: Some(gas_price),
        value: Some(value),
        data: None,
        chain_id: Some(chain_id.as_u64().into()),
    };
    let client = client.clone();
    match client.send_transaction(tx_request, None).await {
        Ok(pending_tx) => {
            let receipt = pending_tx.confirmations(1).await?;
            println!("Transaction Receipt: {:?}", receipt);
        }
        Err(e) => {
            println!("Error sending transaction: {:?}", e);
        }
    }

    Ok(())
}
async fn send_transaction(
    client: &SignerMiddleware<Arc<Provider<Http>>, LocalWallet>,
    typed_tx: TypedTransaction,
) -> Result<(), Box<dyn std::error::Error>> {
    // let signed_tx = client.inner().sign_transaction(&typed_tx, client.address()).await?;
    match client.send_transaction(typed_tx, None).await {
        Ok(pending_tx) => {
            // トランザクションのレシートを取得
            let receipt = pending_tx.confirmations(1).await.unwrap();
            println!("Transaction Receipt: {:?}", receipt);
        }
        Err(e) => {
            println!("Error sending transaction: {:?}", e);
        }
    }

    Ok(())
}