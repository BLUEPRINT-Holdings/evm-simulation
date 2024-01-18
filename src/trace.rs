use anyhow::{anyhow, Result};
use ethers::{
    abi::{self, parse_abi},
    prelude::*,
    types::transaction::eip2930::AccessList,
};
use ethers_providers::Middleware;
use foundry_common::types::ToEthers;
use foundry_evm::revm::primitives::keccak256;
use std::sync::Arc;

use crate::constants::{DEFAULT_CHAIN_ID, DEFAULT_RECIPIENT};

pub struct EvmTracer<M> {
    provider: Arc<M>,
}

impl<M: Middleware + 'static> EvmTracer<M> {
    pub fn new(provider: Arc<M>) -> Self {
        Self { provider }
    }

    pub async fn get_state_diff(
        &self,
        tx: Eip1559TransactionRequest,
        block_number: u64,
    ) -> Result<GethTrace> {
        let trace = self
            .provider
            .debug_trace_call(
                tx,
                Some(BlockId::Number(BlockNumber::Number(block_number.into()))),
                GethDebugTracingCallOptions {
                    tracing_options: GethDebugTracingOptions {
                        disable_storage: None,
                        disable_stack: None,
                        enable_memory: None,
                        enable_return_data: None,
                        tracer: Some(GethDebugTracerType::BuiltInTracer(
                            GethDebugBuiltInTracerType::PreStateTracer,
                        )),
                        tracer_config: None,
                        timeout: None,
                    },
                    state_overrides: None,
                    block_overrides: None,
                },
            )
            .await
            .unwrap();

        Ok(trace)
    }

    pub async fn find_balance_slot(
        &self,
        token: H160,
        owner: H160,
        nonce: U256,
        chain_id: U64,
        block_number: u64,
    ) -> Result<(bool, u32)> {
        // A brute force way of finding the storage slot value of an ERC-20 token
        // Calling balanceOf and tracing the call using "debug_traceCall" will give us access to the
        // storage slot of "balances"
        let erc20_contract = BaseContract::from(
            parse_abi(&["function balanceOf(address) external view returns (uint256)"]).unwrap(),
        );
        let calldata = erc20_contract.encode("balanceOf", owner).unwrap();
        let tx = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(token)),
            from: Some(owner),
            data: Some(calldata.0.into()),
            value: Some(U256::zero()),
            chain_id: Some(chain_id),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            gas: None,
            nonce: Some(nonce),
            access_list: AccessList::default(),
        };
        let trace = self.get_state_diff(tx, block_number).await.unwrap();
        match trace {
            GethTrace::Known(known) => match known {
                GethTraceFrame::PreStateTracer(prestate) => match prestate {
                    PreStateFrame::Default(prestate_mode) => {
                        let token_info =
                            prestate_mode.0.get(&token).ok_or(anyhow!("no token key"))?;
                        let touched_storage =
                            token_info.storage.clone().ok_or(anyhow!("no storage values"))?;
                        for i in 0..20 {
                            let slot = keccak256(&abi::encode(&[
                                abi::Token::Address(owner),
                                abi::Token::Uint(U256::from(i)),
                            ]));
                            match touched_storage.get(&slot.to_ethers()) {
                                Some(_) => {
                                    return Ok((true, i));
                                }
                                None => {}
                            }
                        }
                        Ok((false, 0))
                    }
                    _ => Ok((false, 0)),
                },
                _ => Ok((false, 0)),
            },
            _ => Ok((false, 0)),
        }
    }

    // Checks if possible evil implementation exists. This returns true if the number of mapping(sender => ) is greater than 1 or mapping(owner => ) storage is touched.
    pub async fn check_possible_evil_implementation(
        &self,
        token: H160,
        owner: H160,
        sender: H160,
    ) -> Result<bool> {
        let recipient = *DEFAULT_RECIPIENT;
        let nonce = U256::default();
        let chain_id = DEFAULT_CHAIN_ID;
        let block_number = self.provider.get_block_number().await.unwrap().as_u64();

        // A brute force way of finding the storage slot value of an ERC-20 token
        // Calling transfer and tracing the call using "debug_traceCall" will give us access to the
        // storage slot of "mapping(address => bool) type". But, unfortunately, evm storage mechanism doesn't
        // support the direct detection of the above data type because each slot comsists both of key data type
        // and the index if it's mapping. It means we cannot tell if the slot is mapping(address => bool) or
        // mapping(address => uint256), which is used for balance. So, assuming the ERC20 contract
        // fundamentally has only mapping(address => uint256) as of the type of data has address as a key,
        // we assert the existence of mapping(address => bool) slot by counting the number of slots have the type
        // of data has address as a key.
        let erc20_contract = BaseContract::from(
            parse_abi(&["function transfer(address,uint256) external returns (bool)"]).unwrap(),
        );
        let calldata = erc20_contract.encode("transfer", (recipient, U256::one())).unwrap();

        let from_addresses = if owner.is_zero() { vec![sender] } else { vec![sender, owner] };
        for from_address in from_addresses {
            let is_owner = from_address == owner;
            let tx = Eip1559TransactionRequest {
                to: Some(NameOrAddress::Address(token)),
                from: Some(from_address),
                data: Some(calldata.clone().0.into()),
                value: Some(U256::zero()),
                chain_id: Some(chain_id),
                max_priority_fee_per_gas: None,
                max_fee_per_gas: None,
                gas: None,
                nonce: Some(nonce),
                access_list: AccessList::default(),
            };
            let trace = self.get_state_diff(tx, block_number).await.unwrap();

            let mut count = 0;
            match trace {
                GethTrace::Known(known) => match known {
                    GethTraceFrame::PreStateTracer(prestate) => match prestate {
                        PreStateFrame::Default(prestate_mode) => {
                            let token_info =
                                prestate_mode.0.get(&token).ok_or(anyhow!("no token key"))?;
                            let touched_storage =
                                token_info.storage.clone().ok_or(anyhow!("no storage values"))?;
                            for i in 0..20 {
                                let slot = keccak256(&abi::encode(&[
                                    abi::Token::Address(from_address),
                                    abi::Token::Uint(U256::from(i)),
                                ]));
                                match touched_storage.get(&slot.to_ethers()) {
                                    Some(_) => {
                                        count += 1;
                                        if (is_owner && count.ge(&2)) || (!is_owner && count.ge(&1))
                                        {
                                            return Ok(true);
                                        }
                                    }
                                    None => {
                                        if !owner.is_zero() {
                                            let owner_slot = keccak256(&abi::encode(&[
                                                abi::Token::Address(owner),
                                                abi::Token::Uint(U256::from(i)),
                                            ]));
                                            match touched_storage.get(&owner_slot.to_ethers()) {
                                                Some(_) => {
                                                    return Ok(true);
                                                }
                                                None => {}
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(false)
    }

    pub async fn find_v2_reserves_slot(
        &self,
        pool: H160,
        owner: H160,
        nonce: U256,
        chain_id: U64,
        block_number: u64,
    ) -> Result<(bool, u32)> {
        let v2_pool_contract = BaseContract::from(
            parse_abi(&["function getReserves() external view returns (uint112,uint112,uint32)"])
                .unwrap(),
        );
        let calldata = v2_pool_contract.encode("getReserves", ()).unwrap();
        let tx = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(pool)),
            from: Some(owner),
            data: Some(calldata.0.into()),
            value: Some(U256::zero()),
            chain_id: Some(chain_id),
            max_priority_fee_per_gas: None,
            max_fee_per_gas: None,
            gas: None,
            nonce: Some(nonce),
            access_list: AccessList::default(),
        };
        let trace = self.get_state_diff(tx, block_number).await.unwrap();
        match trace {
            GethTrace::Known(known) => match known {
                GethTraceFrame::PreStateTracer(prestate) => match prestate {
                    PreStateFrame::Default(prestate_mode) => {
                        let token_info =
                            prestate_mode.0.get(&pool).ok_or(anyhow!("no token key"))?;
                        let touched_storage =
                            token_info.storage.clone().ok_or(anyhow!("no storage values"))?;
                        let slot = touched_storage
                            .keys()
                            .next()
                            .ok_or(anyhow!("no slot value in storage"))?;
                        Ok((true, slot.to_low_u64_be() as u32))
                    }
                    _ => Ok((false, 0)),
                },
                _ => Ok((false, 0)),
            },
            _ => Ok((false, 0)),
        }
    }
}
