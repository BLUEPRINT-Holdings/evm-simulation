use alloy_primitives::{Hasher, Keccak};
use anyhow::Result;
use bytes::Bytes as OutputBytes;
use ethers::prelude::BaseContract;
use ethers::types::{Address, Bytes, H160, H256, U256};
use ethers::utils::keccak256;
use ethers_contract::{abigen, EthAbiType};
use ethers_core::abi::{Detokenize, Tokenizable, Tokenize};
use serde::{Deserialize, Serialize};
// use ethers_core::abi::Tokenizable;

// abigen!(GmxV2Reader, "./src/interfaces/abi/gmx_v2/reader.json");
#[derive(Clone)]
pub struct GmxV2ABI {
    pub abi: BaseContract,
}

// NOTE: Consider using Abigen to generate abi bindings
// Also, create enum for u8 values like order_type, decrease_position_swap_type

// For createOrder
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, EthAbiType)]
pub struct CreateOrderParamsAddresses {
    pub receiver: Address,
    pub callback_contract: Address,
    pub ui_fee_receiver: Address,
    pub market: Address,
    pub initial_collateral_token: Address,
    pub swap_path: Vec<Address>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, EthAbiType)]
pub struct CreateOrderParamsNumbers {
    pub size_delta_usd: U256,
    pub initial_collateral_delta_amount: U256,
    pub trigger_price: U256,
    pub acceptable_price: U256,
    pub execution_fee: U256,
    pub callback_gas_limit: U256,
    pub min_output_amount: U256,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, EthAbiType)]
pub struct CreateOrderParams {
    pub addresses: CreateOrderParamsAddresses,
    pub numbers: CreateOrderParamsNumbers,
    pub order_type: u8,                  // Enumの値
    pub decrease_position_swap_type: u8, // Enumの値
    pub is_long: bool,
    pub should_unwrap_native_token: bool,
    pub referral_code: H256,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, EthAbiType)]
pub struct PriceProps {
    pub min: U256,
    pub max: U256,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize, EthAbiType)]
pub struct MarketPrices {
    pub index_token_price: PriceProps,
    pub long_token_price: PriceProps,
    pub short_token_price: PriceProps,
}

#[derive(Debug)]
pub struct TokenInfo {
    pub name: &'static str,
    pub address: &'static str,
    pub decimals: u8,
}

pub enum Token {
    ETH,
    BTC,
    USDC,
}

impl Token {
    pub fn info(&self) -> TokenInfo {
        match self {
            Token::ETH => TokenInfo {
                name: "ETH",
                address: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
                decimals: 18,
            },
            Token::BTC => TokenInfo {
                name: "BTC",
                address: "0x47904963fc8b2340414262125aF798B9655E58Cd",
                decimals: 8,
            },
            Token::USDC => TokenInfo {
                name: "USDC",
                address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
                decimals: 6,
            },
        }
    }

    pub fn from_address(address: &str) -> Option<Token> {
        println!("address: {:?}", address);
        match address {
            "0x82af49447d8a07e3bd95BD0d56f35241523fbab1" => Some(Token::ETH),
            "0x47904963fc8b2340414262125af798B9655e58cd" => Some(Token::BTC),
            "0xaf88d065e77c8cc2239327c5edb3A432268e5831" => Some(Token::USDC),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<Token> {
        match name {
            "ETH" => Some(Token::ETH),
            "BTC" => Some(Token::BTC),
            "USDC" => Some(Token::USDC),
            _ => None,
        }
    }

    pub fn address_from_name(name: &str) -> Option<String> {
        Token::from_name(name).map(|token| token.info().address.to_string())
    }
}


// for getAccountPositioins
#[derive(Debug, Serialize, Deserialize,  EthAbiType)]
pub struct PositionAddresses {
    pub account: Address,
    pub market: Address,
    pub collateral_token: Address,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionProps {
    pub addresses: PositionAddresses,
    pub numbers: PositionNumbers,
    pub flags: PositionFlags,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct AddressInfo {
    pub account: Address,
    pub market: Address,
    pub collateral_token: Address,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionNumbers {
    pub size_in_usd: U256,
    pub size_in_tokens: U256,
    pub collateral_amount: U256,
    pub borrowing_factor: U256,
    pub funding_fee_amount_per_size: U256,
    pub long_token_claimable_funding_amount_per_size: U256,
    pub short_token_claimable_funding_amount_per_size: U256,
    pub increased_at_block: U256,
    pub decreased_at_block: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionFlags {
    pub is_long: bool,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionReferralFees {
    pub referral_code: [u8; 32],
    pub affiliate: Address,
    pub trader: Address,
    pub total_rebate_factor: U256,
    pub trader_discount_factor: U256,
    pub total_rebate_amount: U256,
    pub trader_discount_amount: U256,
    pub affiliate_reward_amount: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionFundingFees {
    pub funding_fee_amount: U256,
    pub claimable_long_token_amount: U256,
    pub claimable_short_token_amount: U256,
    pub latest_funding_fee_amount_per_size: U256,
    pub latest_long_token_claimable_funding_amount_per_size: U256,
    pub latest_short_token_claimable_funding_amount_per_size: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionBorrowingFees {
    pub borrowing_fee_usd: U256,
    pub borrowing_fee_amount: U256,
    pub borrowing_fee_receiver_factor: U256,
    pub borrowing_fee_amount_for_fee_receiver: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionUiFees {
    pub ui_fee_receiver: Address,
    pub ui_fee_receiver_factor: U256,
    pub ui_fee_amount: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionFees {
    pub referral: PositionReferralFees,
    pub funding: PositionFundingFees,
    pub borrowing: PositionBorrowingFees,
    pub ui: PositionUiFees,
    pub collateral_token_price: PriceProps,
    pub position_fee_factor: U256,
    pub protocol_fee_amount: U256,
    pub position_fee_receiver_factor: U256,
    pub fee_receiver_amount: U256,
    pub fee_amount_for_pool: U256,
    pub position_fee_amount_for_pool: U256,
    pub position_fee_amount: U256,
    pub total_cost_amount_excluding_funding: U256,
    pub total_cost_amount: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct ExecutionPriceResult {
    pub price_impact_usd: i128,
    pub price_impact_diff_usd: U256,
    pub execution_price: U256,
}

#[derive(Debug, Serialize, Deserialize, EthAbiType)]
pub struct PositionInfo {
    pub position: PositionProps,
    pub fees: PositionFees,
    pub execution_price_result: ExecutionPriceResult,
    pub base_pnl_usd: i128,
    pub uncapped_base_pnl_usd: i128,
    pub pnl_after_price_impact_usd: i128,
}

// impl GmxV2ABI {
//     pub fn new() -> Self {
//         println!("GmxV2ABI::new");
//         let abi = BaseContract::from(
//             parse_abi(&[
//                 // ExchangeRounter Contract
//                 "function multicall(bytes[] calldata data) external payable virtual returns (bytes[] memory results)",
//                 // "function createOrder(IBaseOrderUtils.CreateOrderParams calldata params) external payable returns (bytes32)",
//                 // "function createOrder(((address,address,address,address,address,address[]),(uint256,uint256,uint256,uint256,uint256,uint256,uint256),uint8,uint8,bool,bool,bytes32)) external payable returns (bytes32)",
//                 "function sendWnt(address receiver, uint256 amount) external payable",

//                 // reader contract: 0x22199a49A999c351eF7927602CFB187ec3cae489
//                 "function getPositionInfo(DataStore dataStore,IReferralStorage referralStorage,bytes32 positionKey,MarketUtils.MarketPrices memory prices,uint256 sizeDeltaUsd,address uiFeeReceiver,bool usePositionSizeAsSizeDeltaUsd,) public view returns (ReaderUtils.PositionInfo memory)",
//             ])
//             .unwrap(),
//         );
//         println!("GmxV2ABI::new end");
//         Self {  abi}
//     }

//     pub fn multicall_input(&self, data: Vec<Bytes>) -> Result<Bytes> {
//         let calldata = self.abi.encode("multicall", data)?;
//         Ok(calldata)
//     }

//     pub fn multicall_output(&self, output: OutputBytes) -> Result<Vec<Bytes>> {
//         let results: Vec<Bytes> = self.abi.decode("multicall", output)?;
//         Ok(results)
//     }

//     pub fn create_order_input(&self, params: CreateOrderParams) -> Result<Bytes> {
//         let calldata = self.abi.encode("createOrder", params)?;
//         Ok(calldata)
//     }

//     pub fn send_wnt_input(&self, receiver: Address, amount: U256) -> Result<Bytes> {
//         let calldata = self.abi.encode("sendWnt", (receiver, amount))?;
//         Ok(calldata)
//     }

//     pub fn get_position_info_input(&self, data_store: Address, referral_storage: Address, position_key: H256, prices: MarketPrices, size_delta_usd: U256, ui_fee_receiver: Address, use_position_size_as_size_delta_usd: bool) -> Result<Bytes> {
//         let calldata = self.abi.encode("getPositionInfo", (data_store, referral_storage, position_key, prices, size_delta_usd, ui_fee_receiver, use_position_size_as_size_delta_usd))?;
//         Ok(calldata)
//     }

//     pub fn get_position_info_output(&self, output: OutputBytes) -> Result<PositionInfo> {
//         let position_info: PositionInfo = self.abi.decode("getPositionInfo", output)?;
//         Ok(position_info)
//     }

// }

use ethers::abi::Token as AbiToken;

pub fn get_position_key(
    account: H160,
    market: H160,
    collateral_token: H160,
    is_long: bool,
) -> [u8; 32] {
    let data_values: Vec<AbiToken> = vec![
        AbiToken::Address(account),
        AbiToken::Address(market),
        AbiToken::Address(collateral_token),
        AbiToken::Bool(is_long),
    ];

    let hash_hex = hash_data(data_values);
    return  hash_hex;
    // Convert hex string to H256
    // hex::decode(hash_hex).expect("Invalid hex string")
}

pub fn hash_data(data_values: Vec<AbiToken>) -> [u8; 32] {
    let encoded_bytes = ethers::abi::encode(&data_values);
    keccak256(encoded_bytes)
    // hex::encode(hash)
}

pub fn claimable_funding_amount_key(market: Address, token: Address, account: Address) -> String {
    let claimable_funding_amount = keccak256(b"CLAIMABLE_FUNDING_AMOUNT");

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&claimable_funding_amount);
    encoded.extend_from_slice(&market.0);
    encoded.extend_from_slice(&token.0);
    encoded.extend_from_slice(&account.0);

    hex::encode(keccak256(&encoded))
}

pub fn account_position_list_key(account: Address) -> H256 {
    let account_position_list = keccak256(b"ACCOUNT_POSITION_LIST");

    let mut encoded = Vec::new();
    encoded.extend_from_slice(&account_position_list);
    encoded.extend_from_slice(&account.0);

    H256::from_slice(&keccak256(encoded))
    // hex::encode(keccak256(&encoded))
}
