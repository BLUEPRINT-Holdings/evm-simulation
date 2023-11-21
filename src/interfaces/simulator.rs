use anyhow::Result;
use bytes::Bytes as OutputBytes;
use ethers::abi::parse_abi;
use ethers::prelude::BaseContract;
use ethers::types::{Bytes, H160, U256};

use crate::constants::{TransferredAmount, SwappedAmount};

#[derive(Clone)]
pub struct SimulatorABI {
    pub abi: BaseContract,
}

impl SimulatorABI {
    pub fn new() -> Self {
        let abi = BaseContract::from(
            parse_abi(&[
                "function buySimulateSwap(uint256,address,address,address) external returns (uint256)",
                "function sellSimulateSwap(uint256,address,address,address) external returns (uint256, uint256)",
                "function getAmountOut(uint256,uint256,uint256) external returns (uint256)",
                "function simpleTransfer(uint256,address) external returns (uint256, uint256)",
            ]).unwrap()
        );
        Self { abi }
    }

    pub fn buy_simulate_swap_input(
        &self,
        amount_in: U256,
        target_pool: H160,
        input_token: H160,
        output_token: H160,
    ) -> Result<Bytes> {
        let calldata = self.abi.encode(
            "buySimulateSwap",
            (amount_in, target_pool, input_token, output_token),
        )?;
        Ok(calldata)
    }

    pub fn buy_simulate_swap_output(&self, output: OutputBytes) -> Result<SwappedAmount> {
        let out = self.abi.decode_output("buySimulateSwap", output)?;
        Ok(out)
    }

    pub fn sell_simulate_swap_input(
        &self,
        amount_in: U256,
        target_pool: H160,
        input_token: H160,
        output_token: H160,
    ) -> Result<Bytes> {
        let calldata = self.abi.encode(
            "sellSimulateSwap",
            (amount_in, target_pool, input_token, output_token),
        )?;
        Ok(calldata)
    }

    pub fn sell_simulate_swap_output(&self, output: OutputBytes) -> Result<(TransferredAmount, SwappedAmount)> {
        let out = self.abi.decode_output("sellSimulateSwap", output)?;
        Ok(out)
    }

    pub fn get_amount_out_input(
        &self,
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
    ) -> Result<Bytes> {
        let calldata = self
            .abi
            .encode("getAmountOut", (amount_in, reserve_in, reserve_out))?;
        Ok(calldata)
    }

    pub fn get_amount_out_output(&self, output: OutputBytes) -> Result<U256> {
        let out = self.abi.decode_output("getAmountOut", output)?;
        Ok(out)
    }

    pub fn simple_transfer_input(
        &self,
        amount: U256,
        sending_token: H160,
    ) -> Result<Bytes> {
        let calldata = self 
            .abi
            .encode("simpleTransfer", (amount, sending_token))?;
        Ok(calldata)
    }
    
    pub fn simple_transfer_output(&self, output: OutputBytes) -> Result<(U256, U256)> {
        let out = self.abi.decode_output("simpleTransfer", output)?;
        Ok(out)
    }
}
