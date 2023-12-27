use alloy_primitives::{Address, U160};
use ethers::types::{Block, BlockId, BlockNumber, H160, H256, U256, U64};
use ethers_providers::Middleware;
use log::info;
use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::pools::Pool;
use crate::simulator::EvmSimulator;
use crate::tokens::{get_implementation, get_token_info, Token};
use crate::trace::EvmTracer;

const WETH_SWAP_AMOUNT: f64 = 0.1;
const TAX_CRITERIA: f64 = 0.1;

#[derive(Debug, Clone)]
pub struct SafeTokens {
    pub weth: H160,
}

impl SafeTokens {
    pub fn new() -> Self {
        Self { weth: H160::from_str("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2").unwrap() }
    }
}

pub struct HoneypotFilter<M> {
    pub simulator: EvmSimulator<M>,
    pub safe_tokens: SafeTokens,
    pub token_info: HashMap<H160, Token>,
    pub safe_token_info: HashMap<H160, Token>,
    pub balance_slots: HashMap<H160, u32>,
    pub honeypot: HashMap<H160, bool>,
    buy_tax: HashMap<H160, f64>,
    sell_tax: HashMap<H160, f64>,
    is_proxy: HashMap<H160, bool>,
}

impl<M: Middleware + 'static> HoneypotFilter<M> {
    pub fn new(provider: Arc<M>, block: Block<H256>) -> Self {
        let owner = H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap();
        let simulator = EvmSimulator::new(provider.clone(), owner, block.number.unwrap());
        let safe_tokens = SafeTokens::new();
        let token_info = HashMap::new();
        let safe_token_info = HashMap::new();
        let balance_slots = HashMap::new();
        let honeypot = HashMap::new();
        let buy_tax = HashMap::new();
        let sell_tax = HashMap::new();
        let is_proxy = HashMap::new();
        Self {
            simulator,
            safe_tokens,
            token_info,
            safe_token_info,
            balance_slots,
            honeypot,
            buy_tax,
            sell_tax,
            is_proxy,
        }
    }

    pub async fn setup(&mut self) {
        // Get safe_token_info using the four following tokens that are widely used as safe tokens
        let provider = &self.simulator.provider;
        let owner = self.simulator.owner;
        let block_number = &self.simulator.block_number;

        let tracer = EvmTracer::new(provider.clone());

        let chain_id = provider.get_chainid().await.unwrap();
        let nonce = self
            .simulator
            .provider
            .get_transaction_count(owner, Some(BlockId::Number(BlockNumber::Number(*block_number))))
            .await
            .unwrap();

        for token in [
            // self.safe_tokens.usdt,
            self.safe_tokens.weth,
            // self.safe_tokens.usdc,
            // self.safe_tokens.dai,
        ] {
            if let std::collections::hash_map::Entry::Vacant(e) = self.safe_token_info.entry(token)
            {
                match tracer
                    .find_balance_slot(
                        token,
                        owner,
                        nonce,
                        U64::from(chain_id.as_u64()),
                        block_number.as_u64(),
                    )
                    .await
                {
                    Ok(slot) => {
                        if slot.0 {
                            self.balance_slots.insert(token, slot.1);
                            let mut info = get_token_info(provider.clone(), token).await.unwrap();
                            match get_implementation(provider.clone(), token, *block_number).await {
                                Ok(implementation) => info.add_implementation(implementation),
                                Err(_) => {}
                            }
                            e.insert(info);
                        }
                    }
                    Err(_) => {}
                }
            }
        }
    }

    pub async fn validate_token(
        &mut self,
        token: H160,
        buy_tax_criteria: Option<U256>,
        sell_tax_criteria: Option<U256>,
    ) {
        let buy_tax_criteria = buy_tax_criteria.unwrap_or(U256::from(5)); // 5%
        let sell_tax_criteria = sell_tax_criteria.unwrap_or(U256::from(10));

        let simulate_tax_res = self.simulator.simulate_tax(token).await;
        let out = match simulate_tax_res {
            Ok(out) => out,
            Err(e) => {
                info!("<Transfer ERROR>: {:?}", e);
                self.honeypot.insert(token, true);
                return;
            }
        };

        if out.0.ge(&buy_tax_criteria) {
            info!("<Send ERROR> Buy Tax Rate: {:?}", out.0);
            self.honeypot.insert(token, true);
        }

        if out.1.ge(&sell_tax_criteria) {
            info!("<Send ERROR> Sell Tax Rate: {:?}", out.1);
            self.honeypot.insert(token, true);
        }

        self.buy_tax.insert(token, out.0.as_u64() as f64 / 100.0);
        self.sell_tax.insert(token, out.1.as_u64() as f64 / 100.0);
    }

    // TODO: Change the arg type so we can directly start executing validation
    //  Now, it's using &Vec<Pool> type to take advantage of the `filter_tokens` function implementation
    // But, this function will be used to validate token which doesn't have pool yet.
    // Use "tokens" with the same type instead of pools
    pub async fn validate_tokens(&mut self, pools: &Vec<Pool>) {
        self.simulator.deploy_simulator();

        let mut tokens = Vec::new();
        // TODO: This part can be cut
        for (_, pool) in pools.iter().enumerate() {
            let token0_is_safe = self.safe_token_info.contains_key(&pool.token0);
            let token1_is_safe = self.safe_token_info.contains_key(&pool.token1);

            // only test for token if it's a match with either of the safe tokens
            if token0_is_safe || token1_is_safe {
                let (_, test_token) = if token0_is_safe {
                    (pool.token0, pool.token1)
                } else {
                    (pool.token1, pool.token0)
                };

                tokens.push(test_token)
            }
        }

        for (_, token) in tokens.iter().enumerate() {
            self.validate_token(*token, None, None).await;
        }
    }

    pub async fn filter_tokens(&mut self, pools: &Vec<Pool>) {
        self.simulator.deploy_simulator();

        for (idx, pool) in pools.iter().enumerate() {
            let token0_is_safe = self.safe_token_info.contains_key(&pool.token0);
            let token1_is_safe = self.safe_token_info.contains_key(&pool.token1);

            if token0_is_safe && token1_is_safe {
                continue;
            }

            // only test for token if it's a match with either of the safe tokens
            if token0_is_safe || token1_is_safe {
                let (safe_token, test_token) = if token0_is_safe {
                    (pool.token0, pool.token1)
                } else {
                    (pool.token1, pool.token0)
                };

                if self.token_info.contains_key(&test_token)
                    || self.honeypot.contains_key(&test_token)
                {
                    // skip if test_tokens was already tested
                    continue;
                }

                // Check if the token contract is proxy
                // If it's proxy contract, we put that into invalid token list without any additional validations
                // NOTE: use big endian to convert H160 bytes into U160
                let is_proxy_contr =
                    self.simulator.is_proxy(Address::from(U160::from_be_bytes(test_token.0)));
                if is_proxy_contr {
                    info!("⚠️ [{}] {} is proxy", idx, test_token);
                    self.honeypot.insert(test_token, true);
                    self.is_proxy.insert(test_token, true);
                    continue;
                }

                // We take extra measures to filter out the pools with too little liquidity
                // Using the below amount to test swaps, we know that there's enough liquidity in the pool
                let amount_in_u32 = 1;
                let mut amount_in_f64 = 1.0;

                if safe_token == self.safe_tokens.weth {
                    amount_in_f64 = WETH_SWAP_AMOUNT;
                }
                // else if safe_token == self.safe_tokens.usdt {
                // amount_in_u32 = 10000;
                // } else if safe_token == self.safe_tokens.usdc {
                // amount_in_u32 = 10000;
                // } else if safe_token == self.safe_tokens.dai {
                // amount_in_u32 = 10000
                // }

                // seed the simulator with some safe token balance
                let safe_token_info = self.safe_token_info.get(&safe_token).unwrap();
                let safe_token_slot = self.balance_slots.get(&safe_token).unwrap();

                self.simulator.set_token_balance(
                    self.simulator.simulator_address,
                    safe_token,
                    safe_token_info.decimals,
                    *safe_token_slot,
                    amount_in_u32,
                );

                info!("✅ [{}] {} -> {:?}", idx, safe_token_info.symbol, test_token);

                let amount_in = if safe_token == self.safe_tokens.weth {
                    U256::from((amount_in_f64 * 10f64.powi(18)) as u64)
                } else {
                    U256::from(amount_in_u32)
                        .checked_mul(U256::from(10).pow(U256::from(safe_token_info.decimals)))
                        .unwrap()
                };

                // Buy Test
                let buy_output = self.simulator.v2_simulate_swap(
                    amount_in,
                    pool.address,
                    safe_token,
                    test_token,
                    true,
                );
                let out = match buy_output {
                    Ok(out) => out,
                    Err(e) => {
                        info!("<BUY ERROR> {:?}", e);
                        self.honeypot.insert(test_token, true);
                        continue;
                    }
                };

                let out_ratio = out.0.checked_sub(out.1).unwrap();
                let buy_tax_rate =
                    out_ratio.checked_mul(U256::from(10000)).unwrap().checked_div(out.0).unwrap();
                let buy_tax_rate = buy_tax_rate.as_u64() as f64 / 10000.0;
                self.buy_tax.insert(test_token, buy_tax_rate);

                if buy_tax_rate < TAX_CRITERIA {
                    // Sell Test
                    let amount_in = out.1;
                    let sell_output = self.simulator.v2_simulate_swap(
                        amount_in,
                        pool.address,
                        test_token,
                        safe_token,
                        true,
                    );
                    let out = match sell_output {
                        Ok(out) => out,
                        Err(e) => {
                            info!("<SELL ERROR> {:?}", e);
                            self.honeypot.insert(test_token, true);
                            continue;
                        }
                    };

                    let out_ratio = out.0.checked_sub(out.1).unwrap();
                    let sell_tax_rate = out_ratio
                        .checked_mul(U256::from(10000))
                        .unwrap()
                        .checked_div(out.0)
                        .unwrap();
                    let sell_tax_rate = sell_tax_rate.as_u64() as f64 / 10000.0;
                    self.sell_tax.insert(test_token, sell_tax_rate);

                    if sell_tax_rate < TAX_CRITERIA {
                        match get_token_info(self.simulator.provider.clone(), test_token).await {
                            Ok(info) => {
                                info!(
                                    "Added safe token info ({}). Total: {:?} tokens",
                                    info.symbol,
                                    self.token_info.len()
                                );
                                self.token_info.insert(test_token, info);
                            }
                            Err(_) => {}
                        }
                    } else {
                        self.honeypot.insert(test_token, true);
                    }
                } else {
                    self.honeypot.insert(test_token, true);
                }
            }
        }
    }

    pub fn get_tax_rate(&self, token: H160) -> (f64, f64) {
        let buy_tax_rate = self.buy_tax.get(&token).unwrap_or(&0.0);
        let sell_tax_rate = self.sell_tax.get(&token).unwrap_or(&0.0);
        (*buy_tax_rate, *sell_tax_rate)
    }

    pub fn is_honeypot(&self, token: H160) -> bool {
        self.honeypot.contains_key(&token)
    }

    pub fn is_proxy(&self, token: H160) -> bool {
        self.is_proxy.contains_key(&token)
    }
}
