use crate::config::USER_AGENT;

use super::Reel;
use anyhow::anyhow;
use headless_chrome::Browser;
use log::{debug, info, trace};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

enum Cmd {
    None,
    NoResponse,
}

pub fn scraper(
    browser: Browser,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<String> {
    let id: String = thread::current()
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("id:{:?}", thread::current().id()));
    let (unlocker, locker) = mpsc::channel();
    let tab = browser.new_tab()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    {
        let t = &tab;
        let tab = tab.clone();
        let id = id.clone();
        t.register_response_handling(
            "reels",
            Box::new(move |res, fetch_body| {
                if !res
                    .response
                    .url
                    .contains("instagram.com/api/v1/clips/user/")
                {
                    return;
                }
                trace!("{id} got /clips/user/ response");
                let mut body = None;
                for i in 0..10 {
                    if let Ok(b) = fetch_body() {
                        body = Some(b.body);
                        break;
                    }
                    if i >= 8 {
                        tab.reload(true, None).unwrap();
                        thread::sleep(Duration::from_secs(10));
                    }
                    thread::sleep(Duration::from_millis(100 * i * 2));
                }
                let body: String = match body {
                    Some(b) => b,
                    None => return unlocker.send(Cmd::NoResponse).unwrap(),
                };
                let body: serde_json::Value = serde_json::from_str(&body).unwrap();
                trace!("{id} sending reels to main thread");
                for reel in body["items"].as_array().unwrap() {
                    tx.send(reel.into()).unwrap();
                }
                unlocker.send(Cmd::None).unwrap();
            }),
        )?;
    }
    loop {
        let account = if let Some(account) = accounts.lock().unwrap().pop() {
            account
        } else {
            break;
        };
        debug!("{id} scraping reels of {account}");
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))?
            .wait_until_navigated()?;
        // FIX: recv_timeout
        if let Cmd::NoResponse = locker.recv().unwrap() {
            return Err(anyhow!("{id} didn't get a response on {account}"));
        };
    }
    tab.deregister_response_handling_all().unwrap();
    info!("{id} finished scraping reels");
    Ok(id.to_string())
}
