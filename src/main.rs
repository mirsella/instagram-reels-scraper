mod browserwithtmpdir;
mod config;
mod insta;
mod slack;

use anyhow::{Context, Result};
use chrono::TimeZone;
use config::Config;
use env_logger::Builder;
use insta::run;
use log::info;
use std::{env, path::PathBuf};
use tempfile::tempdir;

fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("instagram_reels_scraper=trace".into()))
        .init();
    dotenvy::dotenv().context("loading .env")?;
    let config: Config = envy::from_env().context("failed to parse environment variables")?;
    let slack = slack::SlackFileSender::new(&config.slack_token, &config.slack_channel);

    let mut reels = Vec::with_capacity(100);
    let (rx, runner) = run(&config).context("setting up insta")?;
    while let Ok(reel) = rx.recv() {
        reels.push(reel);
    }
    runner.join().unwrap();
    info!("total: {}", reels.len());

    let tmpdir = tempdir()?;
    let date = chrono::Local::now();
    let yesterday = {
        let y = date - chrono::Duration::days(1);
        let y = y.date_naive().and_hms_opt(0, 0, 0).unwrap();
        chrono::Local.from_local_datetime(&y).unwrap()
    };
    println!("yesterday: {}", yesterday);
    let all_path = write_to_file(
        tmpdir.path().join(format!(
            "all-reels-{}.csv",
            date.format("%Y-%m-%d %Hh%M.csv")
        )),
        reels.iter(),
    )?;
    let oneday_path = write_to_file(
        tmpdir.path().join(format!(
            "reels-since-{}",
            yesterday.format("%Y-%m-%d %Hh%M.csv")
        )),
        reels.iter().filter(|reel| reel.date >= yesterday),
    )?;
    slack.send_file(&all_path)?;
    slack.send_file(&oneday_path)?;
    info!("done sending files to slack");
    Ok(())
}

fn write_to_file<'a>(
    path: PathBuf,
    reels: impl Iterator<Item = &'a insta::Reel>,
) -> Result<PathBuf> {
    info!("writing to {}", path.display());
    let mut wtr = csv::Writer::from_path(path.as_path())?;
    for reel in reels {
        wtr.serialize(reel)?;
    }
    Ok(path)
}
