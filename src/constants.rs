use ethers::{
    prelude::Lazy,
    types::{Address, Bytes, U256, U64},
};
use ethers_core::types::H160;
use std::str::FromStr;

pub static WEI: Lazy<U256> = Lazy::new(|| U256::from(10).pow(U256::from(18)));
pub static GWEI: Lazy<U256> = Lazy::new(|| U256::from(10).pow(U256::from(9)));

pub static DEFAULT_SENDER: Lazy<H160> =
    Lazy::new(|| H160::from_str("0x001a06BF8cE4afdb3f5618f6bafe35e9Fc09F187").unwrap());
pub static DEFAULT_RECIPIENT: Lazy<H160> =
    Lazy::new(|| H160::from_str("0x4E17607Fb72C01C280d7b5c41Ba9A2109D74a32C").unwrap());

pub static DEFAULT_CHAIN_ID: U64 = U64::one();
pub static ZERO_ADDRESS: Lazy<Address> =
    Lazy::new(|| Address::from_str("0x0000000000000000000000000000000000000000").unwrap());
pub static DEFAULT_ROUTER_ADDRESS: Lazy<Address> =
    Lazy::new(|| Address::from_str("0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D").unwrap());

pub fn get_env(key: &str) -> String {
    std::env::var(key).unwrap()
}

#[derive(Debug, Clone)]
pub struct Env {
    // pub https_url: String,
    pub wss_url: String,
    pub chain_id: U64,
    pub api_key: String,
}

impl Env {
    pub fn new() -> Self {
        Env {
            // https_url: get_env("HTTPS_URL"),
            wss_url: get_env("WSS_URL"),
            chain_id: U64::from_str(&get_env("CHAIN_ID")).unwrap(),
            api_key: get_env("API_KEY"),
        }
    }
}

pub static SIMULATOR_CODE: Lazy<Bytes> = Lazy::new(|| {
    "0x608060405234801561001057600080fd5b50610ea4806100206000396000f3fe608060405234801561001057600080fd5b50600436106100675760003560e01c8063a4b37afd11610050578063a4b37afd146100ba578063cf62f25b146100cd578063ff53554e146100ea57600080fd5b8063054d50d41461006c57806364bfce6f14610092575b600080fd5b61007f61007a366004610b34565b6100fd565b6040519081526020015b60405180910390f35b6100a56100a0366004610b89565b61027d565b60408051928352602083019190915201610089565b61007f6100c8366004610bd6565b6106f4565b6100d5600a81565b60405163ffffffff9091168152602001610089565b61007f6100f8366004610c12565b6107b1565b6000808411610193576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152602b60248201527f556e697377617056324c6962726172793a20494e53554646494349454e545f4960448201527f4e5055545f414d4f554e5400000000000000000000000000000000000000000060648201526084015b60405180910390fd5b6000831180156101a35750600082115b61022f576040517f08c379a000000000000000000000000000000000000000000000000000000000815260206004820152602860248201527f556e697377617056324c6962726172793a20494e53554646494349454e545f4c60448201527f4951554944495459000000000000000000000000000000000000000000000000606482015260840161018a565b600061023d856103e5610c6d565b9050600061024b8483610c6d565b905060008261025c876103e8610c6d565b6102669190610c84565b90506102728183610c97565b979650505050505050565b6000806102a173ffffffffffffffffffffffffffffffffffffffff8516868861086c565b6000806000808873ffffffffffffffffffffffffffffffffffffffff16630902f1ac6040518163ffffffff1660e01b8152600401606060405180830381865afa1580156102f2573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906103169190610cf0565b506dffffffffffffffffffffffffffff1691506dffffffffffffffffffffffffffff1691508673ffffffffffffffffffffffffffffffffffffffff168873ffffffffffffffffffffffffffffffffffffffff16101561037a57819350809250610381565b8093508192505b50506040517f70a0823100000000000000000000000000000000000000000000000000000000815273ffffffffffffffffffffffffffffffffffffffff888116600483015260009184918916906370a0823190602401602060405180830381865afa1580156103f4573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906104189190610d40565b6104229190610d59565b6040517f054d50d4000000000000000000000000000000000000000000000000000000008152600481018290526024810185905260448101849052909150309063054d50d490606401602060405180830381865afa158015610488573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906104ac9190610d40565b945060646104bb600a82610d6c565b6104cb9063ffffffff1687610c6d565b6104d59190610c97565b6040517f70a0823100000000000000000000000000000000000000000000000000000000815230600482015290955060009073ffffffffffffffffffffffffffffffffffffffff8816906370a0823190602401602060405180830381865afa158015610545573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906105699190610d40565b90506000808873ffffffffffffffffffffffffffffffffffffffff168a73ffffffffffffffffffffffffffffffffffffffff16106105a9578760006105ad565b6000885b604080516000815260208101918290527f022c0d9f00000000000000000000000000000000000000000000000000000000909152919350915073ffffffffffffffffffffffffffffffffffffffff8c169063022c0d9f906106179085908590309060248101610db4565b600060405180830381600087803b15801561063157600080fd5b505af1158015610645573d6000803e3d6000fd5b50506040517f70a0823100000000000000000000000000000000000000000000000000000000815230600482015285925073ffffffffffffffffffffffffffffffffffffffff8c1691506370a0823190602401602060405180830381865afa1580156106b5573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906106d99190610d40565b6106e39190610d59565b965050505050505094509492505050565b600061071773ffffffffffffffffffffffffffffffffffffffff8316848661086c565b6040517f70a0823100000000000000000000000000000000000000000000000000000000815273ffffffffffffffffffffffffffffffffffffffff84811660048301528316906370a0823190602401602060405180830381865afa158015610783573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906107a79190610d40565b90505b9392505050565b60006107d573ffffffffffffffffffffffffffffffffffffffff83163330866108f2565b6040517f70a0823100000000000000000000000000000000000000000000000000000000815230600482015273ffffffffffffffffffffffffffffffffffffffff8316906370a0823190602401602060405180830381865afa15801561083f573d6000803e3d6000fd5b505050506040513d601f19601f820116820180604052508101906108639190610d40565b90505b92915050565b60405173ffffffffffffffffffffffffffffffffffffffff8381166024830152604482018390526108ed91859182169063a9059cbb906064015b604051602081830303815290604052915060e01b6020820180517bffffffffffffffffffffffffffffffffffffffffffffffffffffffff838183161783525050505061093e565b505050565b60405173ffffffffffffffffffffffffffffffffffffffff84811660248301528381166044830152606482018390526109389186918216906323b872dd906084016108a6565b50505050565b600061096073ffffffffffffffffffffffffffffffffffffffff8416836109d4565b905080516000141580156109855750808060200190518101906109839190610e30565b155b156108ed576040517f5274afe700000000000000000000000000000000000000000000000000000000815273ffffffffffffffffffffffffffffffffffffffff8416600482015260240161018a565b606061086383836000846000808573ffffffffffffffffffffffffffffffffffffffff168486604051610a079190610e52565b60006040518083038185875af1925050503d8060008114610a44576040519150601f19603f3d011682016040523d82523d6000602084013e610a49565b606091505b5091509150610a59868383610a63565b9695505050505050565b606082610a7857610a7382610af2565b6107aa565b8151158015610a9c575073ffffffffffffffffffffffffffffffffffffffff84163b155b15610aeb576040517f9996b31500000000000000000000000000000000000000000000000000000000815273ffffffffffffffffffffffffffffffffffffffff8516600482015260240161018a565b50806107aa565b805115610b025780518082602001fd5b6040517f1425ea4200000000000000000000000000000000000000000000000000000000815260040160405180910390fd5b600080600060608486031215610b4957600080fd5b505081359360208301359350604090920135919050565b803573ffffffffffffffffffffffffffffffffffffffff81168114610b8457600080fd5b919050565b60008060008060808587031215610b9f57600080fd5b84359350610baf60208601610b60565b9250610bbd60408601610b60565b9150610bcb60608601610b60565b905092959194509250565b600080600060608486031215610beb57600080fd5b83359250610bfb60208501610b60565b9150610c0960408501610b60565b90509250925092565b60008060408385031215610c2557600080fd5b82359150610c3560208401610b60565b90509250929050565b7f4e487b7100000000000000000000000000000000000000000000000000000000600052601160045260246000fd5b808202811582820484141761086657610866610c3e565b8082018082111561086657610866610c3e565b600082610ccd577f4e487b7100000000000000000000000000000000000000000000000000000000600052601260045260246000fd5b500490565b80516dffffffffffffffffffffffffffff81168114610b8457600080fd5b600080600060608486031215610d0557600080fd5b610d0e84610cd2565b9250610d1c60208501610cd2565b9150604084015163ffffffff81168114610d3557600080fd5b809150509250925092565b600060208284031215610d5257600080fd5b5051919050565b8181038181111561086657610866610c3e565b63ffffffff828116828216039080821115610d8957610d89610c3e565b5092915050565b60005b83811015610dab578181015183820152602001610d93565b50506000910152565b84815283602082015273ffffffffffffffffffffffffffffffffffffffff831660408201526080606082015260008251806080840152610dfb8160a0850160208701610d90565b601f017fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe0169190910160a00195945050505050565b600060208284031215610e4257600080fd5b815180151581146107aa57600080fd5b60008251610e64818460208701610d90565b919091019291505056fea26469706673582212203f410425e6f5ba24d64dad1897ebd80874be790baf37322a3edb3d1710eb21be64736f6c63430008110033"
        .parse()
        .unwrap()
});

// adapted from: https://github.com/gnosis/evm-proxy-detection/blob/main/src/index.ts
pub static EIP_1967_LOGIC_SLOT: &str =
    "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc";
pub static EIP_1967_BEACON_SLOT: &str =
    "0xa3f0ad74e5423aebfd80d3ef4346578335a9a72aeaee59ff6cb3582b35133d50";
pub static OPEN_ZEPPELIN_IMPLEMENTATION_SLOT: &str =
    "0x7050c9e0f4ca769c69bd3a8ef740bc37934f8e2c036e5a723fd8ee048ed3f8c3";
pub static EIP_1882_LOGIC_SLOT: &str =
    "0xc5f16f0fcc639fa48a6947836d9850f504798523bf8c9a3a87d5876cf622bcf7";

pub static IMPLEMENTATION_SLOTS: Lazy<Vec<U256>> = Lazy::new(|| {
    vec![
        U256::from(EIP_1967_LOGIC_SLOT),
        U256::from(EIP_1967_BEACON_SLOT),
        U256::from(OPEN_ZEPPELIN_IMPLEMENTATION_SLOT),
        U256::from(EIP_1882_LOGIC_SLOT),
    ]
});
