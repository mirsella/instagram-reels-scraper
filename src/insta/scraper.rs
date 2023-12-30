use super::Reel;
use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};
use anyhow::Context;
use headless_chrome::{
    browser::tab::ResponseHandler, protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
};

use log::{debug, info, trace, warn};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(20);

pub fn scraper(
    browser: BrowserWithTmpDir,
    accounts: Arc<Mutex<Vec<String>>>,
    main_tx: Sender<Reel>,
) -> anyhow::Result<String> {
    let t = thread::current();
    let id: String = t
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("id:{:?}", t.id()));
    let tab = browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    let (tx, rx) = mpsc::channel::<Reel>();
    let mut handler: ResponseHandler = {
        let id = id.clone();
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
        })
    };
    loop {
        let account = match accounts.lock().unwrap().pop() {
            Some(account) => account,
            None => break,
        };
        debug!("{id}: scraping reels of {account}");
        tab.register_response_handling("reels", handler)?;
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))
            .context("navigate_to")?;
        let mut followers: usize = 0;
        for _ in 0..20 {
            if let Ok(el) = tab.find_element("a[href$='followers/']>span") {
                let v = el
                    .get_attribute_value("title")?
                    .expect("expected title attribute")
                    .replace(',', "")
                    .parse()?;
                followers = v;
            }
            thread::sleep(Duration::from_secs(1));
        }
        if followers == 0 {
            let data = tab.capture_screenshot(Png, None, None, true).unwrap();
            std::fs::write(format!("screenshot-{account}.png"), data).unwrap();
            info!("{id}: screenshot saved to screenshot-{account}.png");
            return Err(anyhow::anyhow!(
                "couldn't get followers count for {account}"
            ));
        }
        info!("{id}: waiting {TIMEOUT:?} for responses");
        while let Ok(mut reel) = rx.recv_timeout(TIMEOUT) {
            reel.set_ratio(followers);
            main_tx.send(reel)?;
        }
        handler = tab.deregister_response_handling("reels")?.unwrap();
    }
    info!("{id}: finished scraping reels");
    Ok(id.to_string())
}
