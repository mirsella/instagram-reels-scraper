use serde::Deserialize;
use std::{collections::HashSet, path::PathBuf};

fn _default_worker() -> usize {
    10
}
fn _default_chromedata() -> PathBuf {
    "/tmp/instagram-reels-scraper".into()
}
fn _default_headless() -> bool {
    true
}

// pub const USER_AGENT: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_1_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1";
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_1) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub struct Config {
    pub insta_user: String,
    pub insta_pass: String,
    pub accounts_type: String,
    pub accounts: HashSet<String>,
    #[serde(default = "_default_worker")]
    pub worker: usize,
    #[serde(default = "_default_headless")]
    pub headless: bool,
    #[serde(default = "_default_chromedata")]
    pub chromedata: PathBuf,
    pub telegram_token: String,
    pub telegram_chat_id: String,
    pub slack_token: String,
    pub slack_channel: String,
}
