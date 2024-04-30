
// NOTE: Consider using Abigen to generate abi bindings
// Also, create enum for u8 values like order_type, decrease_position_swap_type
use anyhow::{anyhow, Result};
use ethers_contract::{abigen, Contract};
use std::{collections::BTreeSet, str::FromStr, sync::Arc};
use ethers::{abi::Abi, prelude::Lazy};
use serde::{Deserialize, Serialize};
use ethers::types::{Address, Block, BlockNumber, H160, H256, U256, Bytes};
use ethers_providers::Middleware;

use crate::{interfaces::gmx::{CreateOrderParams, CreateOrderParamsAddresses, CreateOrderParamsNumbers, GmxV2ABI}, simulator::{EvmSimulator, Tx}};

pub static DATA_STORE: Lazy<H160> = Lazy::new(|| H160::from_str("0xFD70de6b91282D8017aA4E741e9Ae325CAb992d8").unwrap());
pub static EXCHANGE_ROUTER: Lazy<H160> = Lazy::new(|| H160::from_str("0x7C68C7866A64FA2160F78EEaE12217FFbf871fa8").unwrap());
pub static ORDER_VAULT: Lazy<H160> = Lazy::new(|| H160::from_str("0x31eF83a530Fde1B38EE9A18093A333D8Bbbc40D5").unwrap());
pub static READER: Lazy<H160> = Lazy::new(|| H160::from_str("0xf60becbba223EEA9495Da3f606753867eC10d139").unwrap());

// on arbitrum
pub static WETH: Lazy<H160> = Lazy::new(|| H160::from_str("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1").unwrap());

// Define the struct for establishing virtual playground for testing gmx v2 contract
// TODO: make the simulator smaller by modifying the unnecessary abis into optioin
// TODO: Refine properties of GmxPlayground struct including the price decimals, token decimals, etc
abigen!(GmxV2ExchangeRouter, "./src/interfaces/abi/gmx_v2/exchange_router.json");
pub struct GmxPlayground<M: Clone>{
    pub simulator: EvmSimulator<M>,
    // pub gmx_v2: GmxV2ABI, 
    pub exchange_router: GmxV2ExchangeRouter<M>,
}

impl<M: Middleware + 'static + std::clone::Clone> GmxPlayground<M> {
    pub fn new(provider: Arc<M>, block: Block<H256>) -> Self {
        let owner = H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap();
        let simulator = EvmSimulator::new(provider.clone(), owner, block.number.unwrap());         
        let exchange_router = GmxV2ExchangeRouter::new(
            *EXCHANGE_ROUTER,
            provider.clone(),
        );        
        Self {
            simulator,
            // gmx_v2: GmxV2ABI::new(),
            exchange_router,
        }
    }

    pub fn create_order(&self, collateral_token: H160,collateral_amount: U256,size_delta_usd: U256, ) -> Bytes {
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
            swap_path: vec![market_token], // if collateral token is long token of market, put market address here
        };
        let acceptable_price = U256::zero();

        let mut exec_fee = U256::zero();
        if collateral_token == *WETH {
            exec_fee = collateral_amount;
        }
        let create_order_params_numbers = CreateOrderParamsNumbers {
            size_delta_usd: size_delta_usd,
            initial_collateral_delta_amount: collateral_amount,
            trigger_price: U256::zero(), // no need for market order
            acceptable_price: acceptable_price,
            execution_fee: exec_fee,
            callback_gas_limit: U256::zero(),
            min_output_amount: U256::zero(),
        };
        let create_order_params = CreateOrderParams {
            addresses: create_order_params_addresses,
            numbers: create_order_params_numbers,
            order_type: 2,
            decrease_position_swap_type: 0,
            is_long: false,
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
    pub fn create_short_position(&self, collateral_token: H160, collateral_amount: U256, size_delta_usd: U256) -> Result<Vec<Bytes>> {
        let send_wnt = self.send_wnt(collateral_amount);
        let create_order = self.create_order(collateral_token, collateral_amount, size_delta_usd);
        let calldata = self.exchange_router.encode("multicall",vec![send_wnt, create_order]).unwrap();
        let tx = Tx {
            caller: self.simulator.owner,
            transact_to: *EXCHANGE_ROUTER,
            data: calldata.0,
            gas_limit: 1000000,
            value: collateral_amount,
        };
        let mut simulator = self.simulator.clone();
        let result = match simulator._call(tx, true) {
            Ok(result) => result,
            Err(e) => return Err(e),
        };
        println!("result: {:?}", result);
        let outputs = self.exchange_router.decode("multicall", result.output).unwrap();
        Ok(outputs)
    }

}

// TODO: understand Middleware trait and how its used as a generic type
