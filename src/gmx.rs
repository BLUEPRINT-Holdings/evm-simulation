// NOTE: Consider using Abigen to generate abi bindings
// Also, create enum for u8 values like order_type, decrease_position_swap_type
use anyhow::{anyhow, Result};
use ethers::types::{Address, Block, BlockNumber, Bytes, H160, H256, U256};
use ethers::{
    abi::Abi,
    prelude::Lazy,
    types::{Eip1559TransactionRequest, NameOrAddress},
    utils::keccak256,
};
use ethers_contract::{abigen, Contract};
use ethers_providers::Middleware;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, ops::Mul, str::FromStr, sync::Arc};

use crate::interfaces::gmx::PositionInfo as IPositionInfo;
use crate::{
    interfaces::gmx::{
        get_position_key, CreateOrderParams, CreateOrderParamsAddresses, CreateOrderParamsNumbers,
        GmxV2ABI, MarketPrices, PriceProps, Token,
    },
    simulator::{EvmSimulator, Tx},
};

pub static DATA_STORE: Lazy<H160> =
    Lazy::new(|| H160::from_str("0xFD70de6b91282D8017aA4E741e9Ae325CAb992d8").unwrap());
pub static EXCHANGE_ROUTER: Lazy<H160> =
    Lazy::new(|| H160::from_str("0x7C68C7866A64FA2160F78EEaE12217FFbf871fa8").unwrap());
pub static REFERRAL_STORAGE: Lazy<H160> =
    Lazy::new(|| H160::from_str("0xe6fab3f0c7199b0d34d7fbe83394fc0e0d06e99d").unwrap());
pub static ORDER_VAULT: Lazy<H160> =
    Lazy::new(|| H160::from_str("0x31eF83a530Fde1B38EE9A18093A333D8Bbbc40D5").unwrap());
pub static READER: Lazy<H160> =
    Lazy::new(|| H160::from_str("0xf60becbba223EEA9495Da3f606753867eC10d139").unwrap());

// on arbitrum
pub static WETH: Lazy<H160> =
    Lazy::new(|| H160::from_str("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1").unwrap());

const USD_SCALE_FACTOR: u8 = 30; // Scaling factor for USD values

// Define the struct for establishing virtual playground for testing gmx v2 contract
// TODO: make the simulator smaller by modifying the unnecessary abis into optioin
abigen!(GmxV2ExchangeRouter, "./src/interfaces/abi/gmx_v2/exchange_router.json");

// NOTE: Modified Position.Pros struct name to Position.PositionPros
//       to avoid name confliction between Price.Pros
//       The reason why I modified Position's struct name is because Price.Pros
//       is already defined in exchange_router.json

// NOTE: Also, I eliminated the functions uses Address,Numbers, Flag struct from reader contract abi set
//       because they conflicts with exchange router contract abi set and are not considerd to use here
abigen!(GmxV2Reader, "./src/interfaces/abi/gmx_v2/reader.json");

pub struct GmxPlayground<M: Clone> {
    pub simulator: EvmSimulator<M>,
    // pub gmx_v2: GmxV2ABI,
    pub exchange_router: GmxV2ExchangeRouter<M>,
    pub reader: GmxV2Reader<M>,
}

impl<M: Middleware + 'static + std::clone::Clone> GmxPlayground<M> {
    pub fn new(provider: Arc<M>, block: Block<H256>) -> Self {
        // NOTE: Change the owner address to the address you want to use
        let owner = H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap();
        let simulator = EvmSimulator::new(provider.clone(), owner, block.number.unwrap());
        let exchange_router = GmxV2ExchangeRouter::new(*EXCHANGE_ROUTER, provider.clone());
        let reader = GmxV2Reader::new(*READER, provider.clone());
        Self {
            simulator,
            // gmx_v2: GmxV2ABI::new(),
            exchange_router,
            reader,
        }
    }

    pub async fn create_order(
        &self,
        collateral_token: H160,
        collateral_amount: U256,
        size_delta_usd: f64,
        long: bool,
        exec_fee: U256,
    ) -> Bytes {
        // define market token based on collateral token
        // if collateral token is weth, then market token is 0x70d95587d40A2caf56bd97485aB3Eec10Bee6336
        // if collateral token is btc, then market token is // TODO: fill
        let mut market_token = H160::zero();
        if collateral_token == *WETH {
            market_token = H160::from_str("0x70d95587d40A2caf56bd97485aB3Eec10Bee6336").unwrap();
        }
        let create_order_params_addresses = CreateOrderParamsAddresses {
            receiver: self.simulator.owner,
            callback_contract: H160::zero(),
            ui_fee_receiver: H160::zero(),
            market: market_token,
            initial_collateral_token: collateral_token,
            // swap path is empty
            swap_path: vec![], // if collateral token is long token of market, put market address here
        };

        // Directly define ETH as quote token for price fetch for now
        // let current_index_token_price = fetch_token_price("ETH".to_string()).await.unwrap();

        // NOTE: We can modify the acceptable price by changing the percentage in production
        // let acceptable_price = U256::from_str(&current_index_token_price.max_price_full).unwrap().checked_mul(U256::from(95)).unwrap().checked_div(U256::from(100)).unwrap();
        let price_decimal = 30 - Token::from_name("ETH").unwrap().info().decimals;
        let acceptable_price = expand_decimals(3000.0, price_decimal);
        let size_in_usd_in_decimals = expand_decimals(size_delta_usd, USD_SCALE_FACTOR);
        let create_order_params_numbers = CreateOrderParamsNumbers {
            size_delta_usd: size_in_usd_in_decimals,
            initial_collateral_delta_amount: collateral_amount,
            trigger_price: U256::zero(), // no need for market order
            acceptable_price,
            execution_fee: exec_fee,
            callback_gas_limit: U256::zero(),
            min_output_amount: U256::zero(),
        };
        let create_order_params = CreateOrderParams {
            addresses: create_order_params_addresses,
            numbers: create_order_params_numbers,
            order_type: 2,
            decrease_position_swap_type: 0,
            is_long: long,
            should_unwrap_native_token: false,
            referral_code: H256::zero(),
        };
        self.exchange_router.encode("createOrder", (create_order_params,)).unwrap()
    }

    pub fn send_wnt(&self, amount: U256) -> Bytes {
        let receiver = *ORDER_VAULT;
        let calldata = self.exchange_router.encode("sendWnt", (receiver, amount)).unwrap();
        calldata
    }

    // TODO: create position calling multicall containing the logic of sendWnt, createOrder
    pub async fn create_short_position_tx(
        &mut self,
        collateral_token: &str,
        collateral_amount: f64,
        size_delta_usd: f64,
        exec_fee: U256,
    ) -> Result<Eip1559TransactionRequest> {
        let collateral_token_addr =
            H160::from_str(&Token::from_name(collateral_token).unwrap().info().address).unwrap();
        let collateral_token_deciamls = Token::from_name(collateral_token).unwrap().info().decimals;
        let collateral_amount = expand_decimals(collateral_amount, collateral_token_deciamls);
        let sending_amount = collateral_amount.checked_add(exec_fee).unwrap();
        let send_wnt = self.send_wnt(sending_amount);
        let create_order = self
            .create_order(collateral_token_addr, collateral_amount, size_delta_usd, false, exec_fee)
            .await;

        // approve Collateral token for router contract
        // NOTE: This is not necessary for ETH
        // let approve_tx = self.simulator.approve(collateral_token_addr, *EXCHANGE_ROUTER, true)?;

        let calldata =
            self.exchange_router.multicall(vec![send_wnt, create_order]).calldata().unwrap();
        // let tx = Tx {
        //     caller: self.simulator.owner,
        //     transact_to: *EXCHANGE_ROUTER,
        //     data: calldata.0,
        //     gas_limit: 1000000,
        //     value: collateral_amount,
        // };

        // let result = self.simulator._call(tx, true)?;
        // println!("result: {:?}", result);
        // NOTE: createOrder response is meaningless

        let priority_fee: U256 = U256::from(100000000);
        let gas_price: U256 = self.simulator.provider.get_gas_price().await.unwrap();
        let max_fee_per_gas: U256 = gas_price + priority_fee;
        let gas_estimate: U256 = U256::from(4000000);
        let gas_limit: U256 = gas_estimate + 100000; // Buffer
        let nonce = self
            .simulator
            .provider
            .get_transaction_count(self.simulator.owner, None)
            .await
            .unwrap();
        let chain_id = self.simulator.provider.get_chainid().await.unwrap(); // 42161

        let typed_tx: Eip1559TransactionRequest =
            ethers::types::transaction::eip1559::Eip1559TransactionRequest {
                from: Some(self.simulator.owner),
                to: Some(NameOrAddress::Address(*EXCHANGE_ROUTER)).into(),
                nonce: Some(nonce),
                max_priority_fee_per_gas: Some(priority_fee),
                max_fee_per_gas: Some(max_fee_per_gas),
                gas: Some(gas_limit),
                value: Some(sending_amount),
                data: Some(calldata),
                access_list: ethers::types::transaction::eip2930::AccessList(Vec::new()),
                chain_id: Some(chain_id.as_u64().into()),
            };

        Ok(typed_tx)
    }

    // Just return the calldata for now
    pub fn claim_funding_fees(&self, market_token_addr: H160) -> Bytes {
        let market_token_addrs = vec![market_token_addr, market_token_addr];
        // TODO: Define market_token_address and long, short addreses in the same struct
        // and extract long and short addresses from the struct
        let long_token_addr = *WETH;
        let short_token_addr = H160::from_str(Token::from_name("USDC").unwrap().info().address).unwrap();
        let long_short_token_adrdrs = vec![long_token_addr, short_token_addr];
        let receiver = self.simulator.owner;
        let calldata = self
            .exchange_router
            .encode("claimFundingFees", (market_token_addrs,long_short_token_adrdrs, receiver))
            .unwrap();
        calldata
    }

    pub async fn fill_tx_fields(&mut self, calldata: Bytes) -> Result<Eip1559TransactionRequest> {
        let priority_fee: U256 = U256::from(100000000);
        let gas_price: U256 = self.simulator.provider.get_gas_price().await.unwrap();
        let max_fee_per_gas: U256 = gas_price + priority_fee;
        let gas_estimate: U256 = U256::from(4000000);
        let gas_limit: U256 = gas_estimate + 100000; // Buffer
        let nonce = self
            .simulator
            .provider
            .get_transaction_count(self.simulator.owner, None)
            .await
            .unwrap();
        
        let chain_id = self.simulator.provider.get_chainid().await.unwrap(); // 42161

        let typed_tx: Eip1559TransactionRequest =
            ethers::types::transaction::eip1559::Eip1559TransactionRequest {
                from: Some(self.simulator.owner),
                to: Some(NameOrAddress::Address(*EXCHANGE_ROUTER)).into(),
                nonce: Some(nonce),
                max_priority_fee_per_gas: Some(priority_fee),
                max_fee_per_gas: Some(max_fee_per_gas),
                gas: Some(gas_limit),
                value: Some(U256::zero()),
                data: Some(calldata),
                access_list: ethers::types::transaction::eip2930::AccessList(Vec::new()),
                chain_id: Some(chain_id.as_u64().into()),
            };

        Ok(typed_tx)
    }

    pub fn get_account_positions(&mut self) -> Bytes {
        let data_store = *DATA_STORE;
        let owner: Address = self.simulator.owner;
        let start_index = U256::from(0);
        let end_index = U256::from(10);
        let query_call = self
            .reader
            .encode("getAccountPositions", (data_store, owner, start_index, end_index))
            .unwrap();
        return query_call;
        // let tx = Tx {
        //     caller: self.simulator.owner,
        //     transact_to: *READER,
        //     data: query_call.0,
        //     gas_limit: 1000000,
        //     value: U256::zero(),
        // };
        // let result = self.simulator._call(tx, true);
        // let outputs: Vec<PositionProps> = self.reader.decode("getAccountPositions", result.unwrap().output).unwrap();
        // println!("Account Positions: {:?}", outputs);
    }

    pub async fn get_position_info_calldata(&mut self, market_token: H160) -> Bytes {
        let data_store = *DATA_STORE;
        let referral_storage = *REFERRAL_STORAGE;
        // Assume the position is short
        // That is represented by is_long = false at last
        let collateral_token = *WETH;
        let position_key =
            get_position_key(self.simulator.owner, market_token, collateral_token, false);
        // println!("Position Key: {:?}", position_key);
        let size_delta_usd = U256::zero();
        let ui_fee_receiver = H160::zero();
        let use_position_size_as_size_delta_usd = true;
        // Index Token is Long Token
        let eth_prices = fetch_token_price("ETH".to_string()).await.unwrap();
        let usdc_prices = fetch_token_price("USDC".to_string()).await.unwrap();
        // index prices can be any numbers because it doens't affect the important results
        let index_prices = PriceProps { min: U256::from_dec_str("3865").unwrap(), max: U256::from_dec_str("3890").unwrap() };
        let market_prices = MarketPrices {
            index_token_price: index_prices,
            long_token_price: PriceProps{ min: eth_prices.min_price_full, max: eth_prices.max_price_full},
            short_token_price: PriceProps { min: usdc_prices.min_price_full, max: usdc_prices.max_price_full},
        };
        // print!("Market Prices: {:?}", market_prices);
        let query_call = self
            .reader
            .encode(
                "getPositionInfo",
                (
                    data_store,
                    referral_storage,
                    position_key,
                    market_prices,
                    size_delta_usd,
                    ui_fee_receiver,
                    use_position_size_as_size_delta_usd,
                ),
            )
            .unwrap();
        // let tx = Tx {
        //     caller: self.simulator.owner,
        //     transact_to: *READER,
        //     data: query_call.clone().0,
        //     gas_limit: 1000000,
        //     value: U256::zero(),
        // };
        // Execution on simulation
        // let result = self.simulator._call(tx, false);
        // println!("result: {:?}", result);

        return query_call
        // let outputs = self.reader.decode("getPositionInfo", result.unwrap().output).unwrap();
        // println!("Position Info: {:?}", outputs);
    }

    pub async fn get_accrued_funding_fee_in_usd(&self, position_info: IPositionInfo) -> f64 {
        let long_token_funding_amount = position_info.fees.funding.claimable_long_token_amount;
        let short_token_funding_amount = position_info.fees.funding.claimable_short_token_amount;
        // Assume we are using collateral in long token
        let collateral_token_addr = format!("{:?}", position_info.position.addresses.collateral_token);
        let collateral_token_info = Token::from_address(&collateral_token_addr).unwrap().info();
        // println!("Collateral Token Info: {:?}", collateral_token_info);
        let collateral_token_decimals = collateral_token_info.decimals;
        let collateral_token_name = collateral_token_info.name;
        let long_token_prices = fetch_token_price(collateral_token_name.to_string()).await.unwrap();
        println!("Long Token Prices: {:?}", long_token_prices);
        let price_decimal = 30 - collateral_token_decimals;
        let long_token_funding_in_usd = mul_div(
            long_token_funding_amount,
            long_token_prices.min_price_full,
            U256::from(10).pow(U256::from(collateral_token_decimals)),
        ).unwrap();
        let long_token_funding_in_usd = u256_to_f64(long_token_funding_in_usd) / 10_f64.powf(price_decimal.into());
        println!("Long Token Funding in USD: {:?}", long_token_funding_in_usd);

        let short_token_funding_in_usd = u256_to_f64(short_token_funding_amount) / 10_f64.powf(6.0);
        let funding_in_usd = long_token_funding_in_usd + short_token_funding_in_usd;

        funding_in_usd
    }
}

// TODO: understand Middleware trait and how its used as a generic type
// funtion to expand decimals of f64 value
pub fn expand_decimals(num: f64, decimals: u8) -> U256 {
    let num = num * 10_f64.powi(decimals as i32);
    U256::from(num as u128)
}

#[derive(Deserialize, Debug)]
pub struct PriceData {
    pub id: String,
    #[serde(rename = "minBlockNumber")]
    pub min_block_number: Option<u64>,
    #[serde(rename = "minBlockHash")]
    pub min_block_hash: Option<String>,
    #[serde(rename = "oracleDecimals")]
    pub oracle_decimals: Option<u8>,
    #[serde(rename = "tokenSymbol")]
    pub token_symbol: String,
    #[serde(rename = "tokenAddress")]
    pub token_address: String,
    #[serde(rename = "minPrice")]
    pub min_price: Option<String>,
    #[serde(rename = "maxPrice")]
    pub max_price: Option<String>,
    pub signer: Option<String>,
    pub signature: Option<String>,
    #[serde(rename = "signatureWithoutBlockHash")]
    pub signature_without_block_hash: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: Option<String>,
    #[serde(rename = "minBlockTimestamp")]
    pub min_block_timestamp: Option<u64>,
    #[serde(rename = "oracleKeeperKey")]
    pub oracle_keeper_key: Option<String>,
    #[serde(rename = "maxBlockTimestamp")]
    pub max_block_timestamp: Option<u64>,
    #[serde(rename = "maxBlockNumber")]
    pub max_block_number: Option<u64>,
    #[serde(rename = "maxBlockHash")]
    pub max_block_hash: Option<String>,
    #[serde(rename = "maxPriceFull")]
    pub max_price_full: Option<String>,
    #[serde(rename = "minPriceFull")]
    pub min_price_full: Option<String>,
    #[serde(rename = "oracleKeeperRecordId")]
    pub oracle_keeper_record_id: Option<String>,
    #[serde(rename = "oracleKeeperFetchType")]
    pub oracle_keeper_fetch_type: Option<String>,
    #[serde(rename = "oracleType")]
    pub oracle_type: Option<String>,
    pub blob: Option<String>,
}
#[derive(Deserialize)]
pub struct ApiResponse {
    #[serde(rename = "signedPrices")]
    pub signed_prices: Vec<PriceData>,
}

#[derive(Deserialize, Debug)]
pub struct TokenPriceFromApiResponse {
    pub token_symbol: String,
    pub min_price_full: U256,
    pub max_price_full: U256,
}

#[derive(Deserialize)]
pub struct GasPriceResponse {
    pub result: String,
}

pub async fn fetch_token_price(
    mut index_token: String,
) -> Result<TokenPriceFromApiResponse, Box<dyn std::error::Error>> {
    let url: &str = "https://arbitrum-api.gmxinfra.io/signed_prices/latest";

    // Get the raw response
    let response = reqwest::get(url).await?;
    let response_text = response.text().await?;
    if index_token == "WBTC" {
        index_token = "WBTC.b".to_string();
    }

    // Deserialize the response text to ApiResponse
    let response_json: ApiResponse = serde_json::from_str(&response_text)?;

    // Find the relevant price data for the specified token
    for price_data in response_json.signed_prices {
        if price_data.token_symbol == index_token {
            let min_price = price_data.min_price_full.unwrap_or_default();
            let max_price = price_data.max_price_full.unwrap_or_default();

            return Ok(TokenPriceFromApiResponse {
                token_symbol: price_data.token_symbol,
                min_price_full: U256::from_dec_str(&min_price).unwrap(),
                max_price_full: U256::from_dec_str(&max_price).unwrap(),
            });
        }
    }

    Err("Token not found in price data".into())
}

// pub fn calculate_accured_funding_in_usd(long_funding_fee_per_size: U256, short_funding_fee_per_size: U256, long_token: String) -> BigUint {
//     let long_funding_fee_per_size = long_funding_fee_per_size;
//     let short_funding_fee_per_size = short_funding_fee_per_size;
//     let long_token_prices = fetch_token_price(long_token).unwrap();
//     let short_token_prices = fetch_token_price("USDC".to_string()).unwrap();


//     let funding_fee_per_size = if long_token == "ETH" {
//         long_funding_fee_per_size
//     } else {
//         short_funding_fee_per_size
//     };
//     funding_fee_per_size
// }

pub fn mul_div(value: U256, numerator: U256, denominator: U256) -> Option<U256> {
    // Perform the multiplication
    let result = value.checked_mul(numerator).unwrap();
    // Perform the division and return the result
    result.checked_div(denominator)
}


pub fn u256_to_f64(value: U256) -> f64 {
    let mut result = 0.0;
    let base: f64 = 2.0_f64.powf(64.0); // 2^64
    for i in 0..4 {
        let part = (value >> (i * 64)).low_u64() as f64;
        result += part * base.powi(i as i32);
    }
    result
}