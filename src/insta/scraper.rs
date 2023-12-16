use super::Reel;
use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};
use anyhow::Context;
use log::{debug, info, trace, warn};
use std::{
    sync::{mpsc::Sender, Arc, Mutex},
    thread,
    time::Duration,
};

pub fn scraper(
    browser: BrowserWithTmpDir,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<String> {
    let id: String = thread::current()
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("id:{:?}", thread::current().id()));
    let tab = browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    {
        let id = id.clone();
        tab.register_response_handling(
            "reels",
            Box::new(move |res, fetch_body| {
                if !res.response.url.contains("clips/user") {
                    return;
                }
                trace!("{id}: got clips response");
                thread::sleep(Duration::from_secs(4));
                let body = match fetch_body() {
                    Ok(body) => body.body,
                    Err(e) => {
                        warn!("{id}: couldn't get a body from response: {e}");
                        return;
                    }
                };
                let body: serde_json::Value = serde_json::from_str(&body).unwrap();
                trace!("{id}: got body, sending reels to main thread");
                for reel in body["items"].as_array().unwrap() {
                    tx.send(reel.into()).unwrap();
                }
            }),
        )?;
    }
    loop {
        let account = match accounts.lock().unwrap().pop() {
            Some(account) => account,
            None => break,
        };
        debug!("{id}: scraping reels of {account}");
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))
            .context("navigate_to")?;
        // .wait_until_navigated()
        // .context("wait_until_navigated")?;
        info!("{id}: waiting 30s for responses");
        thread::sleep(Duration::from_secs(30));
    }
    tab.deregister_response_handling_all().unwrap();
    info!("{id}: finished scraping reels");
    Ok(id.to_string())
}
