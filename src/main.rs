mod browserwithtmpdir;
mod config;
mod insta;
mod slack;
mod telegram;

use anyhow::{bail, Context, Result};
use chrono::NaiveTime;
use config::Config;
use env_logger::Builder;
use indexmap::IndexSet;
use log::info;
use spreadsheet_ods::{Sheet, WorkBook};
use std::{env, path::PathBuf};
use tempfile::tempdir;

use crate::telegram::Telegram;

fn main() -> Result<()> {
    Builder::new()
        .parse_filters(&env::var("RUST_LOG").unwrap_or("instagram_reels_scraper=trace".into()))
        .init();
    dotenvy::dotenv().context("loading .env")?;
    let config: Config = envy::from_env().context("failed to parse environment variables")?;
    let telegram = Telegram::new(&config.telegram_token, &config.telegram_chat_id);
    info!(
        "scraping `{}` reels from {} accounts: {:?}",
        config.accounts_type,
        config.accounts.len(),
        config.accounts
    );
    let slack = slack::SlackFileSender::new(&config.slack_token, &config.slack_channel);

    let mut reels = IndexSet::with_capacity(300);
    let (rx, runner) = insta::run(&config).context("setting up insta")?;
    while let Ok(reel) = rx.recv() {
        reels.insert(reel);
    }
    runner.join().unwrap();
    reels.sort_unstable_by(|a, b| {
        a.ratio
            .unwrap_or_default()
            .total_cmp(&b.ratio.unwrap_or_default())
    });
    reels.reverse();
    info!("total: {} reels", reels.len());
    if reels.is_empty() {
        telegram.send("instagram_reels_scraper: no reels found")?;
        bail!("no reels found");
    }

    let today = chrono::Local::now()
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let yesterday = today - chrono::Duration::days(1);
    let tmpdir = tempdir()?;
    let path = tmpdir.path().join(format!(
        "{} reels {}.ods",
        config.accounts_type,
        today.format("%d-%m-%Y")
    ));

    let mut wb = WorkBook::new_empty();

    let mut sh_today = Sheet::new("today");
    write_to_sheet(&mut sh_today, reels.iter().filter(|r| r.date >= today))?;
    wb.push_sheet(sh_today);

    let mut sh_yesterday = Sheet::new("yesterday");
    write_to_sheet(
        &mut sh_yesterday,
        reels
            .iter()
            .filter(|r| r.date <= today && r.date >= yesterday),
    )?;
    wb.push_sheet(sh_yesterday);

    let mut sh_month = Sheet::new("last month");
    write_to_sheet(&mut sh_month, &reels)?;
    wb.push_sheet(sh_month);

    spreadsheet_ods::write_ods(&mut wb, &path)?;
    slack.send_file(&path)?;
    Ok(())
}

fn write_to_sheet<'a>(
    sh: &mut Sheet,
    reels: impl IntoIterator<Item = &'a insta::Reel>,
) -> Result<()> {
    let fields = [
        "url",
        "ratio",
        "account",
        "like",
        "comments",
        "views",
        "duration",
        "paid partnership",
        "date",
        "caption",
    ];
    fields.iter().enumerate().for_each(|(i, f)| {
        sh.set_value(0, i as u32, *f);
    });
    for (i, reel) in reels.into_iter().enumerate() {
        let i = i as u32 + 1;
        let url = &reel.url;
        let formula = format!(r#"=HYPERLINK("{url}";"url")"#);
        sh.set_formula(i, 0, formula);
        sh.set_col_width(0, spreadsheet_ods::Length::In(0.40));
        sh.set_value(i, 1, *reel.ratio.unwrap_or_default());
        sh.set_value(i, 2, &reel.account);
        sh.set_col_width(2, spreadsheet_ods::Length::In(1.));
        sh.set_value(i, 3, reel.like as u32);
        sh.set_value(i, 4, reel.comments as u32);
        sh.set_col_width(4, spreadsheet_ods::Length::In(0.4));
        sh.set_value(i, 5, reel.views.unwrap_or_default() as u32);
        sh.set_value(i, 6, format!("{:?}", reel.duration));
        sh.set_col_width(6, spreadsheet_ods::Length::In(0.6));
        sh.set_value(i, 7, reel.paid_partnership);
        sh.set_col_width(7, spreadsheet_ods::Length::In(0.5));
        sh.set_value(i, 8, &reel.date.format("%d-%m-%Y %H:%M:%S").to_string());
        sh.set_col_width(8, spreadsheet_ods::Length::In(1.35));
        sh.set_value(i, 9, &reel.caption);
        sh.set_col_width(9, spreadsheet_ods::Length::In(15.));
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
    fn main_on_test_channel() {
        dotenvy::dotenv().ok();
        let test_id = env::var("test_slack_channel").unwrap();
        env::set_var("slack_channel", test_id);
        super::main().unwrap();
    }
}
