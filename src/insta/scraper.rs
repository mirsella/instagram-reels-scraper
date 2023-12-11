use super::Reel;
use headless_chrome::Tab;
use log::info;
use serde_json::Value;
use std::{
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
    time::Duration,
};

pub fn scraper(
    tab: Arc<Tab>,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<()> {
    tab.register_response_handling(
        "reels",
        Box::new(move |res, fetch_body| {
            if !res
                .response
                .url
                .contains("instagram.com/api/v1/clips/user/")
            {
                return;
            }
            let body = fetch_body()
                .unwrap_or_else(|_| {
                    thread::sleep(Duration::from_millis(1000));
                    fetch_body().unwrap()
                })
                .body;
            let body: serde_json::Value = serde_json::from_str(&body).unwrap();
            for reel in body["items"].as_array().unwrap() {
                tx.send(reel.into()).unwrap();
            }
        }),
    )?;
    // while let Some(account) = accounts.lock()?.pop() {
    // }
    tab.navigate_to("https://www.instagram.com/le.media.positif/reels/")?
        .wait_until_navigated()?;
    thread::sleep(Duration::from_secs(10));
    Ok(())
}
