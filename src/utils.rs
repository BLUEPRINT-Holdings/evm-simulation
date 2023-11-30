use alloy_primitives::{Keccak, Hasher as _};
use anyhow::{self, Result};
use ethers_core::{types::{H160, U256}, utils::rlp::RlpStream};
use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;

pub fn setup_logger() -> Result<()> {
    let colors = ColoredLevelConfig {
        trace: Color::Cyan,
        debug: Color::Magenta,
        info: Color::Green,
        warn: Color::Red,
        error: Color::BrightRed,
        ..ColoredLevelConfig::new()
    };

    fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{}[{}] {}",
                chrono::Local::now().format("[%H:%M:%S]"),
                colors.color(record.level()),
                message
            ))
        })
        .chain(std::io::stdout())
        .level(log::LevelFilter::Error)
        .level_for("evm_simulation", LevelFilter::Info)
        .apply()?;

    Ok(())
}

pub fn calculate_contract_address(sender: H160, nonce: U256) -> H160 {
    let mut stream = RlpStream::new_list(2);
    stream.append(&sender);
    stream.append(&nonce);

    let mut keccak = Keccak::v256();
    keccak.update(stream.as_raw());

    let mut result = [0u8; 32];
    keccak.finalize(&mut result);

    H160::from_slice(&result[12..])
}
