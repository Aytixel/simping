use std::error::Error;

use async_signal::{Signal, Signals};
use macro_rules_attribute::apply;
use smol::prelude::*;
use smol_macros::main;

use crate::config::Config;

mod config;

const CONFIG_PATH: &str = "/etc/simping/config.toml";
const RTM_PROTOCOL: u8 = 69;

#[apply(main!)]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut config = Config::load(CONFIG_PATH).await?;
    let mut signals = Signals::new(&[Signal::Int, Signal::Term, Signal::Hup])?;

    log::info!("Started successfully");

    while let Some(signal) = signals.next().await {
        match signal? {
            Signal::Int | Signal::Term => break,
            Signal::Hup => {
                config = Config::load(CONFIG_PATH).await?;
            }
            _ => {}
        }
    }

    log::info!("Stopped gracefully");

    Ok(())
}
