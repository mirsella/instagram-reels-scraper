mod config;
mod insta;

use anyhow::{Context, Result};
use config::Config;
use env_logger::Builder;
use insta::setup;
use std::{env, thread};

fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("instagram_reels_scraper=trace".into()))
        .init();
    dotenvy::dotenv().context("loading .env")?;
    let config: Config = envy::from_env().context("failed to parse environment variables")?;
    setup(&config).context("setting up insta")?;
    thread::park();
    Ok(())
}
