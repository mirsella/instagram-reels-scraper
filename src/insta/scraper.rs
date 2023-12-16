use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};

use super::Reel;
use anyhow::{anyhow, Context};
use log::{debug, info, trace};
use std::{
    sync::{
        mpsc::{self, RecvTimeoutError, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

enum Cmd {
    None,
    NoBody,
}

pub fn scraper(
    browser: BrowserWithTmpDir,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<String> {
    let id: String = thread::current()
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("id:{:?}", thread::current().id()));
    let (unlocker, locker) = mpsc::channel();
    let tab = browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    {
        let id = id.clone();
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
                trace!("{id} got /clips/user/ response");
                let mut body = None;
                for i in 1..10 {
                    if let Ok(b) = fetch_body() {
                        body = Some(b.body);
                        break;
                    }
                    debug!("{id} waiting for body {i}/10");
                    thread::sleep(Duration::from_secs(i));
                }
                let body: String = match body {
                    Some(b) => b,
                    None => {
                        let res = unlocker.send(Cmd::NoBody);
                        debug!("{id} unlocking with NoBody: {res:?}");
                        return;
                    }
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
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))
            .context("navigate_to")?;
        match locker.recv_timeout(Duration::from_secs(60)) {
            Ok(Cmd::None) => (),
            Ok(Cmd::NoBody) => {
                tab.deregister_response_handling_all().unwrap();
                return Err(anyhow!("{id} couldn't get a body from {account}"));
            }
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {
                tab.deregister_response_handling_all().unwrap();
                return Err(anyhow!(
                    "{id} timeout while waiting for responde handler for {account}"
                ));
            }
        }
    }
    tab.deregister_response_handling_all().unwrap();
    info!("{id} finished scraping reels");
    Ok(id.to_string())
}
