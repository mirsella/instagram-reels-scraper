mod browserwithtmpdir;
mod config;
mod insta;

use anyhow::{Context, Result};
use config::Config;
use env_logger::Builder;
use insta::run;
use log::info;
use std::env;

fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("instagram_reels_scraper=trace".into()))
        .init();
    dotenvy::dotenv().context("loading .env")?;
    let config: Config = envy::from_env().context("failed to parse environment variables")?;

    let mut reels = Vec::with_capacity(100);
    let (rx, runner) = run(&config).context("setting up insta")?;
    while let Ok(reel) = rx.recv() {
        reels.push(reel);
    }
    runner.join().unwrap();
    info!("total: {}", reels.len());
    Ok(())
}
