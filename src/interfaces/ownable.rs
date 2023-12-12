use anyhow::Result;
use bytes::Bytes as OutputBytes;
use ethers::abi::parse_abi;
use ethers::prelude::BaseContract;
use ethers::types::{Bytes, H160};

#[derive(Clone)]
pub struct OwnableABI {
    pub abi: BaseContract,
}

impl OwnableABI {
    pub fn new() -> Self {
        let abi = BaseContract::from(
            parse_abi(&[
                "function owner() public view virtual returns (address)",
            ]).unwrap(),
        );
        Self { abi }
    }

    pub fn owner_input(&self) -> Result<Bytes> {
        let calldata = self.abi.encode("owner", ())?;
        Ok(calldata)
    }

    pub fn owner_output(&self, output: OutputBytes) -> Result<H160> {
        let out = self.abi.decode_output("owner", output)?;
        Ok(out)
    }
}