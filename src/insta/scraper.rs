use super::Reel;
use headless_chrome::Tab;
use log::{debug, trace};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub fn scraper(
    tab: Arc<Tab>,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<()> {
    let (unlocker, locker) = mpsc::channel();
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
            trace!("got /clips/user/ response");
            let body = fetch_body()
                .unwrap_or_else(|_| {
                    thread::sleep(Duration::from_millis(1000));
                    fetch_body().unwrap()
                })
                .body;
            let body: serde_json::Value = serde_json::from_str(&body).unwrap();
            trace!("sending reels to main thread");
            for reel in body["items"].as_array().unwrap() {
                tx.send(reel.into()).unwrap();
            }
            unlocker.send(()).unwrap();
        }),
    )?;
    while let Some(account) = accounts.lock().unwrap().pop() {
        debug!("scraping reels of {}", account);
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))?
            .wait_until_navigated()?;
        locker.recv().unwrap();
    }
    Ok(())
}
