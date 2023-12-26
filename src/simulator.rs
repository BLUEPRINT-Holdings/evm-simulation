use alloy_primitives::{Address, B256, U256 as aU256};
use anyhow::{anyhow, Result};
use bytes::Bytes;
use ethers::abi;
use ethers::types::{Transaction, H160, U256, U64};
use ethers_providers::Middleware;
use foundry_evm::{
    fork::{BlockchainDb, BlockchainDbMeta, SharedBackend},
    revm::{
        db::{CacheDB, Database},
        primitives::{
            keccak256, AccountInfo, Bytecode, ExecutionResult, Output, TransactTo, KECCAK_EMPTY,
            U256 as rU256,
        },
        EVM,
    },
};
use foundry_utils::types::{ToAlloy, ToEthers};
use std::{collections::BTreeSet, str::FromStr, sync::Arc};

use crate::constants::{IMPLEMENTATION_SLOTS, SIMULATOR_CODE};
use crate::interfaces::ownable::OwnableABI;
use crate::interfaces::{pool::V2PoolABI, simulator::SimulatorABI, token::TokenABI};
use crate::tokens::get_token_info;
use crate::trace::EvmTracer;

#[derive(Clone)]
pub struct EvmSimulator<M> {
    pub provider: Arc<M>,
    pub owner: H160,
    pub evm: EVM<CacheDB<SharedBackend>>,
    pub block_number: U64,

    pub token: TokenABI,
    pub v2_pool: V2PoolABI,
    pub simulator: SimulatorABI,
    pub ownable: OwnableABI,

    pub simulator_address: H160,
}

#[derive(Debug, Clone)]
pub struct Tx {
    pub caller: H160,
    pub transact_to: H160,
    pub data: Bytes,
    pub value: U256,
    pub gas_limit: u64,
}

#[derive(Debug, Clone)]
pub struct TxResult {
    pub output: Bytes,
    pub gas_used: u64,
    pub gas_refunded: u64,
}

#[derive(Debug, Clone)]
pub struct SimpleTransferResult {
    pub transfered_amount: U256,
    pub return_amount: U256,
    pub gas_used: u64,
}

impl<M: Middleware + 'static> EvmSimulator<M> {
    pub fn new(provider: Arc<M>, owner: H160, block_number: U64) -> Self {
        let shared_backend = SharedBackend::spawn_backend_thread(
            provider.clone(),
            BlockchainDb::new(
                BlockchainDbMeta {
                    cfg_env: Default::default(),
                    block_env: Default::default(),
                    hosts: BTreeSet::from(["".to_string()]),
                },
                None,
            ),
            Some(block_number.into()),
        );
        let db = CacheDB::new(shared_backend);

        let mut evm = EVM::new();
        evm.database(db);

        evm.env.cfg.limit_contract_code_size = Some(0x100000);
        evm.env.cfg.disable_block_gas_limit = true;
        evm.env.cfg.disable_base_fee = true;

        evm.env.block.number = rU256::from(block_number.as_u64() + 1);

        Self {
            provider,
            owner,
            evm,
            block_number,

            token: TokenABI::new(),
            v2_pool: V2PoolABI::new(),
            simulator: SimulatorABI::new(),
            ownable: OwnableABI::new(),

            simulator_address: H160::from_str("0x4E17607Fb72C01C280d7b5c41Ba9A2109D74a32C")
                .unwrap(),
        }
    }

    pub fn inject_db(&mut self, db: CacheDB<SharedBackend>) {
        self.evm.database(db);
    }

    pub fn run_pending_tx(&mut self, tx: &Transaction) -> Result<TxResult> {
        // We simply need to commit changes to the DB
        self.evm.env.tx.caller = tx.from.0.into();
        self.evm.env.tx.transact_to = TransactTo::Call(tx.to.unwrap_or_default().0.into());
        self.evm.env.tx.data = tx.input.0.clone().into();
        self.evm.env.tx.value = tx.value.to_alloy();
        self.evm.env.tx.chain_id = tx.chain_id.map(|id| id.as_u64());
        self.evm.env.tx.gas_limit = tx.gas.as_u64();

        match tx.transaction_type {
            Some(U64([0])) => {
                self.evm.env.tx.gas_price = tx.gas_price.unwrap_or_default().to_alloy()
            }
            Some(_) => {
                self.evm.env.tx.gas_priority_fee =
                    tx.max_priority_fee_per_gas.map(|mpf| mpf.to_alloy());
                self.evm.env.tx.gas_price = tx.max_fee_per_gas.unwrap_or_default().to_alloy();
            }
            None => self.evm.env.tx.gas_price = tx.gas_price.unwrap_or_default().to_alloy(),
        }

        let result = match self.evm.transact_commit() {
            Ok(result) => result,
            Err(e) => return Err(anyhow!("EVM call failed: {:?}", e)),
        };

        let output = match result {
            ExecutionResult::Success { gas_used, gas_refunded, output, .. } => match output {
                Output::Call(o) => TxResult { output: o.into(), gas_used, gas_refunded },
                Output::Create(o, _) => TxResult { output: o.into(), gas_used, gas_refunded },
            },
            ExecutionResult::Revert { gas_used, output } => {
                return Err(anyhow!("EVM REVERT: {:?} / Gas used: {:?}", output, gas_used))
            }
            ExecutionResult::Halt { reason, .. } => return Err(anyhow!("EVM HALT: {:?}", reason)),
        };

        Ok(output)
    }

    pub fn _call(&mut self, tx: Tx, commit: bool) -> Result<TxResult> {
        self.evm.env.tx.caller = tx.caller.to_alloy();
        self.evm.env.tx.transact_to = TransactTo::Call(tx.transact_to.to_alloy());
        self.evm.env.tx.data = tx.data.into();
        self.evm.env.tx.value = tx.value.to_alloy();
        self.evm.env.tx.gas_limit = 5000000;

        let result;

        if commit {
            result = match self.evm.transact_commit() {
                Ok(result) => result,
                Err(e) => return Err(anyhow!("EVM call failed: {:?}", e)),
            };
        } else {
            let ref_tx =
                self.evm.transact_ref().map_err(|e| anyhow!("EVM staticcall failed: {:?}", e))?;
            result = ref_tx.result;
        }

        let output = match result {
            ExecutionResult::Success { gas_used, gas_refunded, output, .. } => match output {
                Output::Call(o) => TxResult { output: o.into(), gas_used, gas_refunded },
                Output::Create(o, _) => TxResult { output: o.into(), gas_used, gas_refunded },
            },
            ExecutionResult::Revert { gas_used, output } => {
                return Err(anyhow!("EVM REVERT: {:?} / Gas used: {:?}", output, gas_used))
            }
            ExecutionResult::Halt { reason, .. } => return Err(anyhow!("EVM HALT: {:?}", reason)),
        };

        Ok(output)
    }

    pub fn staticcall(&mut self, tx: Tx) -> Result<TxResult> {
        self._call(tx, false)
    }

    pub fn call(&mut self, tx: Tx) -> Result<TxResult> {
        self._call(tx, true)
    }

    pub async fn simulate_tax(&mut self, token: H160) -> Result<(U256, U256)> {
        self.deploy_simulator();

        let amount_u32 = 10000;
        let token_info = get_token_info(self.provider.clone(), token).await.unwrap();
        let amount = U256::from(amount_u32)
            .checked_mul(U256::from(10).pow(U256::from(token_info.decimals)))
            .unwrap();

        let tracer = EvmTracer::new(self.provider.clone());
        let chain_id = self.provider.get_chainid().await.unwrap();
        let token_slot = tracer
            .find_balance_slot(
                token,
                self.owner,
                U256::zero(),
                U64::from(chain_id.as_u64()),
                self.block_number.as_u64(),
            )
            .await
            .unwrap();

        self.set_token_balance(self.owner, token, token_info.decimals, token_slot.1, amount_u32);

        self.approve(token, self.simulator_address, true).unwrap();

        // Transfer Test
        let transfer_result = self.simple_transfer(amount, token, true)?;

        // TODO: Make a validation against gas cost
        // let gas_cost = out.1;

        let send_transfered_amount = transfer_result.transfered_amount;
        let reducted_out_amount = amount.checked_sub(send_transfered_amount).unwrap();
        let buy_tax_rate =
            reducted_out_amount.checked_mul(U256::from(100)).unwrap().checked_div(amount).unwrap();

        let return_transfered_amount = transfer_result.return_amount;

        let reducted_out_amount =
            send_transfered_amount.checked_sub(return_transfered_amount).unwrap();
        let sell_tax_rate = reducted_out_amount
            .checked_mul(U256::from(100))
            .unwrap()
            .checked_div(send_transfered_amount)
            .unwrap();

        // NOTE: should we return gas comsumption?
        Ok((buy_tax_rate, sell_tax_rate))
    }

    pub fn get_eth_balance(&mut self) -> U256 {
        let acc = self.evm.db.as_mut().unwrap().basic(self.owner.to_alloy()).unwrap().unwrap();
        acc.balance.to_ethers()
    }

    pub fn set_eth_balance(&mut self, balance: u32) {
        let user_balance =
            rU256::from(balance).checked_mul(rU256::from(10).pow(rU256::from(18))).unwrap();
        let user_info = AccountInfo::new(user_balance, 0, KECCAK_EMPTY, Bytecode::default());
        self.evm.db.as_mut().unwrap().insert_account_info(self.owner.to_alloy(), user_info);
    }

    // ERC-20 Token functions
    pub fn set_token_balance(
        &mut self,
        account: H160,
        token: H160,
        decimals: u8,
        slot: u32,
        balance: u32,
    ) {
        let slot = keccak256(abi::encode(&[
            abi::Token::Address(account),
            abi::Token::Uint(U256::from(slot)),
        ]));
        let target_balance =
            rU256::from(balance).checked_mul(rU256::from(10).pow(rU256::from(decimals))).unwrap();
        self.evm
            .db
            .as_mut()
            .unwrap()
            .insert_account_storage(token.to_alloy(), slot.into(), target_balance)
            .unwrap();
    }

    pub fn token_balance_of(&mut self, token: H160, account: H160) -> Result<U256> {
        let calldata = self.token.balance_of_input(account)?;
        let value = self.staticcall(Tx {
            caller: self.owner,
            transact_to: token,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 0,
        })?;
        let out = self.token.balance_of_output(value.output)?;
        Ok(out)
    }

    // V2 Pool functions
    pub fn set_v2_pool_reserves(&mut self, pool: H160, reserves: rU256) {
        let slot = rU256::from(8);
        self.evm
            .db
            .as_mut()
            .unwrap()
            .insert_account_storage(pool.to_alloy(), slot, reserves)
            .unwrap();
    }

    pub fn v2_pool_get_reserves(&mut self, pool: H160) -> Result<(u128, u128, u32)> {
        let calldata = self.v2_pool.get_reserves_input()?;
        let value = self.staticcall(Tx {
            caller: self.owner,
            transact_to: pool,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 0,
        })?;
        let out = self.v2_pool.get_reserves_output(value.output)?;
        Ok(out)
    }

    // Simulator functions
    pub fn deploy_simulator(&mut self) {
        let code = Bytecode::new_raw((*SIMULATOR_CODE.0).into());
        let contract_info =
            AccountInfo::new(rU256::ZERO, 0, B256::from_slice(&keccak256(code.bytes())[..]), code);
        self.evm
            .db
            .as_mut()
            .unwrap()
            .insert_account_info(self.simulator_address.to_alloy(), contract_info);
    }

    pub fn v2_simulate_swap(
        &mut self,
        amount_in: U256,
        target_pool: H160,
        input_token: H160,
        output_token: H160,
        commit: bool,
    ) -> Result<(U256, U256)> {
        let calldata = self.simulator.v2_simulate_swap_input(
            amount_in,
            target_pool,
            input_token,
            output_token,
        )?;
        let tx = Tx {
            caller: self.owner,
            transact_to: self.simulator_address,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        };
        let value = if commit { self.call(tx)? } else { self.staticcall(tx)? };
        let out = self.simulator.v2_simulate_swap_output(value.output)?;
        Ok(out)
    }

    pub fn get_amount_out(
        &mut self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<U256> {
        let calldata = self.simulator.get_amount_out_input(amount_in, reserve_in, reserve_out)?;
        let value = self.staticcall(Tx {
            caller: self.owner,
            transact_to: self.simulator_address,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        })?;
        let out = self.simulator.get_amount_out_output(value.output)?;
        Ok(out)
    }

    pub fn is_proxy(&mut self, token: Address) -> bool {
        let mut is_proxy = false;

        for slot in IMPLEMENTATION_SLOTS.iter() {
            let impl_addr = self.evm.db.as_mut().unwrap().storage(token, slot.to_alloy()).unwrap();
            if impl_addr.count_zeros() != 256 {
                is_proxy = true;
            }
        }

        is_proxy
    }

    pub fn approve(&mut self, token: H160, spender: H160, commit: bool) -> Result<bool> {
        let calldata = self.token.approve_input(spender)?;

        let tx = Tx {
            caller: self.owner.into(),
            transact_to: token,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        };

        let value = if commit { self.call(tx)? } else { self.staticcall(tx)? };
        let out = self.token.approve_output(value.output)?;
        Ok(out)
    }

    // This function will fail with the message of
    // "missing or wrong function selector"
    pub fn transfer(
        &mut self,
        token: H160,
        recipient: H160,
        amount: U256,
        commit: bool,
    ) -> Result<bool> {
        let calldata = self.token.transfer_input(recipient, amount)?;

        let tx = Tx {
            caller: self.owner.into(),
            transact_to: token,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        };

        let value = if commit { self.call(tx)? } else { self.staticcall(tx)? };
        let out = self.token.transfer_output(value.output)?;
        Ok(out)
    }

    pub fn simple_transfer(
        &mut self,
        amount: U256,
        sending_token: H160,
        commit: bool,
    ) -> Result<SimpleTransferResult> {
        let calldata = self.simulator.simple_transfer_input(amount, sending_token)?;

        let tx = Tx {
            caller: self.owner,
            transact_to: self.simulator_address,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        };

        let value = if commit { self.call(tx)? } else { self.staticcall(tx)? };

        let out = self.simulator.simple_transfer_output(value.output)?;

        Ok(SimpleTransferResult {
            transfered_amount: out.0,
            return_amount: out.1,
            gas_used: value.gas_used,
        })
    }

    // It calls owner() view function in the format of Ownable interface from OpenZeppelin
    // You can interpret the result as follows:
    // If the result is Address, but not zero, then, it is the address stored as owner for Ownable part
    // It it's Zero Address, it means the former owner called renounce to throw away the ownership
    // If the result if Err and the execution is reverted, the token_contract doesn't inherit the Ownable
    pub fn check_owner(&mut self, token_contract: H160) -> Result<H160> {
        let calldata = self.ownable.owner_input()?;

        let tx = Tx {
            caller: self.owner,
            transact_to: token_contract,
            data: calldata.0,
            value: U256::zero(),
            gas_limit: 5000000,
        };

        let value = self.staticcall(tx)?;
        let out = self.ownable.owner_output(value.output)?;
        Ok(out)
    }

    // Check the existence of an admin address for ERC20 contract.
    // The reason why it is limited to ERC20 contract is because we assume the specific storage slot management.
    // In ERC20, the standard implementation and the plugin parts are highly limited. Hence, we assume if the ERC20 contract
    // has address type storage slot in the contract, it would be the address who can do administrative tasks as an admin.
    pub fn check_admin(&mut self, token_contract: H160) -> Result<(bool, Vec<H160>)> {
        let token_contract = token_contract.to_alloy();
        let mut possible_admins = Vec::new();
        for i in 0..15 {
            let res = self.evm.db.as_mut().unwrap().storage(token_contract, aU256::from(i))?;

            // Convert Uint<256, 4> to big-endian bytes and take the values from 12 to the last
            let mut be_vec = res.as_le_slice().to_vec();
            be_vec.reverse();
            if be_vec[..12].iter().filter(|x| **x == 0).count() != 12 {
                continue;
            }

            let owner = H160::from_slice(&&be_vec[12..]);
            if !owner.is_zero() {
                possible_admins.push(owner);
            }
        }

        let possible_address_storage = !possible_admins.is_empty();
        Ok((possible_address_storage, possible_admins))
    }
}
