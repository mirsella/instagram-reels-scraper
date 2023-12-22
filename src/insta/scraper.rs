use super::Reel;
use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};
use anyhow::Context;
use log::{debug, error, info, trace, warn};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub fn scraper(
    browser: BrowserWithTmpDir,
    accounts: Arc<Mutex<Vec<String>>>,
    main_tx: Sender<Reel>,
) -> anyhow::Result<String> {
    let id: String = thread::current()
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("id:{:?}", thread::current().id()));
    let tab = browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    let (tx, rx) = mpsc::channel::<Reel>();
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
        if let Err(e) = tab.wait_for_element("a[href$='followers/']>span") {
            error!("{id}: couldn't find followers element for {account}: {e}");
            accounts.lock().unwrap().insert(0, account);
            continue;
        };
        let followers: usize = tab
            .wait_for_element("a[href$='followers/']>span")
            .context("wait_for_element on followers")?
            .get_attribute_value("title")?
            .unwrap()
            .replace(',', "")
            .parse()?;
        info!("{id}: waiting 30s for responses");
        while let Ok(mut reel) = rx.recv_timeout(Duration::from_secs(30)) {
            reel.set_ratio(followers);
            main_tx.send(reel)?;
        }
    }
    tab.deregister_response_handling_all().unwrap();
    info!("{id}: finished scraping reels");
    Ok(id.to_string())
}
