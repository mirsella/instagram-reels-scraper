mod browserwithtmpdir;
mod config;
mod insta;
mod slack;
mod telegram;

use anyhow::{Context, Result};
use chrono::TimeZone;
use config::Config;
use env_logger::Builder;
use indexmap::IndexSet;
use insta::run;
use log::info;
use spreadsheet_ods::{Sheet, WorkBook};
use std::{env, path::PathBuf};
use tempfile::tempdir;

fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("instagram_reels_scraper=trace".into()))
        .init();
    dotenvy::dotenv().context("loading .env")?;
    let config: Config = envy::from_env().context("failed to parse environment variables")?;
    info!(
        "scraping `{}` reels from {} accounts: {:?}",
        config.accounts_type,
        config.accounts.len(),
        config.accounts
    );
    let slack = slack::SlackFileSender::new(&config.slack_token, &config.slack_channel);

    let mut reels = IndexSet::with_capacity(300);
    let (rx, runner) = run(&config).context("setting up insta")?;
    while let Ok(reel) = rx.recv() {
        reels.insert(reel);
    }
    reels.sort_unstable_by(|a, b| {
        a.ratio
            .unwrap_or_default()
            .total_cmp(&b.ratio.unwrap_or_default())
    });
    reels.reverse();
    runner.join().unwrap();
    info!("total: {}", reels.len());
    assert!(!reels.is_empty());

    let date = chrono::Local::now();
    let yesterday = {
        let y = date - chrono::Duration::days(1);
        let y = y.date_naive().and_hms_opt(0, 0, 0).unwrap();
        chrono::Local.from_local_datetime(&y).unwrap()
    };
    let tmpdir = tempdir()?;
    let path = tmpdir.path().join(format!(
        "{} reels {}.ods",
        config.accounts_type,
        date.format("%Y-%m-%d %Hh%M")
    ));

    let mut wb = WorkBook::new_empty();
    wb.push_sheet(Sheet::new(format!(
        "since {}",
        yesterday.format("%m-%d 00h00")
    )));
    write_to_sheet(
        wb.sheet_mut(0),
        reels.iter().filter(|r| r.date >= yesterday),
    )?;
    wb.push_sheet(Sheet::new("all"));
    write_to_sheet(wb.sheet_mut(1), &reels)?;
    spreadsheet_ods::write_ods(&mut wb, &path)?;
    slack.send_file(&path)?;
    Ok(())
}

fn write_to_sheet<'a>(
    sh: &mut Sheet,
    reels: impl IntoIterator<Item = &'a insta::Reel>,
) -> Result<()> {
    let fields = [
        "link", "ratio", "account", "like", "comments", "views", "duration", "date", "caption",
    ];
    fields.iter().enumerate().for_each(|(i, f)| {
        sh.set_value(0, i as u32, *f);
    });
    for (i, reel) in reels.into_iter().enumerate() {
        let i = i as u32 + 1;
        let link = &reel.link;
        let formula = format!("=HYPERLINK(\"{}\";\"{}\")", link, link);
        sh.set_col_width(0, spreadsheet_ods::Length::In(3f64));
        sh.set_formula(i, 0, formula);
        sh.set_value(i, 1, *reel.ratio.unwrap_or_default());
        sh.set_col_width(2, spreadsheet_ods::Length::In(1.5));
        sh.set_value(i, 2, &reel.account);
        sh.set_value(i, 3, reel.like as u32);
        sh.set_value(i, 4, reel.comments as u32);
        sh.set_value(i, 5, reel.views.unwrap_or_default() as u32);
        sh.set_value(i, 6, format!("{:?}", reel.duration));
        sh.set_col_width(7, spreadsheet_ods::Length::In(1.35));
        sh.set_value(i, 7, &reel.date.format("%Y-%m-%d %H:%M:%S").to_string());
        sh.set_col_width(8, spreadsheet_ods::Length::In(15f64));
        sh.set_value(i, 8, &reel.caption);
    }
    Ok(())
}

fn _write_to_csv<'a>(
    path: PathBuf,
    reels: impl IntoIterator<Item = &'a insta::Reel>,
) -> Result<PathBuf> {
    info!("writing to {}", path.display());
    let mut wtr = csv::Writer::from_path(path.as_path())?;
    for reel in reels {
        wtr.serialize(reel)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::env;

    #[test]
    fn main_on_private_channel() {
        dotenvy::dotenv().ok();
        let test_id = env::var("test_slack_channel").unwrap();
        env::set_var("slack_channel", test_id);
        super::main().unwrap();
    }
}
