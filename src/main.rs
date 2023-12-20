mod browserwithtmpdir;
mod config;
mod insta;

use anyhow::{Context, Result};
use config::Config;
use env_logger::Builder;
use insta::run;
use log::info;
use std::time::Duration;
use std::{env, path::PathBuf};
use tempfile::tempdir;

fn main() -> Result<()> {
    let date = chrono::Local::now();
    println!("date: {date}, {date:?}, {date:#?}, {date:#}");
    println!("date: {}", date.to_string());
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
    let tmpdir = tempdir()?;
    let date = chrono::Local::now();
    let yesterday = date - chrono::Duration::days(1);
    let all_path = write_to_file(
        tmpdir.path().join(format!("all-reels-${date}")),
        reels.iter(),
    )?;
    let oneday_path = write_to_file(
        tmpdir.path().join(format!("reels-since-${yesterday}")),
        reels.iter().filter(|reel| reel.date >= yesterday),
    )?;
    // TODO: send all_path + oneday_path to slack
    println!("all_path: {:?}", all_path);
    println!("oneday_path: {:?}", oneday_path);
    std::thread::sleep(Duration::from_secs(60));
    Ok(())
}

fn write_to_file<'a>(
    path: PathBuf,
    reels: impl Iterator<Item = &'a insta::Reel>,
) -> Result<PathBuf> {
    let mut wtr = csv::Writer::from_path(path.as_path())?;
    for reel in reels {
        wtr.serialize(reel)?;
    }
    Ok(path)
}
