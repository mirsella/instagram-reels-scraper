use super::Reel;
use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};
use anyhow::{bail, Context};
use headless_chrome::browser::tab::ResponseHandler;

use log::{debug, info, trace};
use regex::Regex;
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

const TIMEOUT: Duration = Duration::from_secs(20);

enum Data {
    Reel(Reel),
    Followers(usize),
}

// fn wait_for_body<I, T: Fn() -> anyhow::Result<I>>(f: T) -> I {
fn wait_for_body<I>(f: impl Fn() -> anyhow::Result<I>) -> I {
    loop {
        if let Ok(body) = f() {
            return body;
        };
        thread::sleep(Duration::from_millis(100));
    }
}

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
    tab.set_user_agent(USER_AGENT, None, None)?;
    let re = Regex::new(r#""edge_followed_by":\{"count":(\d+)\}"#).unwrap();
    let (tx, rx) = mpsc::channel::<Data>();
    let mut handler: ResponseHandler = {
        let id = id.clone();
        Box::new(move |res, fetch_body| {
            if res.response.url.contains("api/v1/users/web_profile_info") {
                let body = wait_for_body(fetch_body);
                let count = re
                    .captures(&body.body)
                    .expect("a edge_followed_by in the response")
                    .get(1)
                    .unwrap();
                tx.send(Data::Followers(count.as_str().parse().unwrap()))
                    .unwrap();
                return;
            }
            if !res.response.url.contains("graphql") {
                return;
            }
            let body = wait_for_body(fetch_body).body;
            if !body.contains("xdt_api__v1__clips__user__connection_v2") {
                return;
            }
            let body: serde_json::Value = serde_json::from_str(&body).expect("valid json");
            trace!("{id}: got body, sending reels to main thread");
            let reels = body["data"]["xdt_api__v1__clips__user__connection_v2"]["edges"]
                .as_array()
                .expect("a array");
            for (i, reel) in reels.iter().enumerate() {
                if i >= 3 {
                    break;
                }
                let reel = &reel["node"]["media"];
                tx.send(Data::Reel(reel.into())).unwrap();
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
        tab.wait_until_navigated()?;
        info!("{id}: waiting {TIMEOUT:?} for responses");
        let mut reels = vec![];
        let mut followers = 0;
        while let Ok(reel) = rx.recv_timeout(TIMEOUT) {
            match reel {
                Data::Reel(reel) => reels.push(reel),
                Data::Followers(n) => {
                    debug!("{id}: got followers count: {n}");
                    followers = n
                }
            }
        }
        if followers == 0 {
            bail!("{id}: couldn't get followers count");
        }
        for mut reel in reels {
            reel.set_followers(followers);
            reel.set_account(Some(account.clone()));
            main_tx.send(reel)?;
        }
        handler = tab.deregister_response_handling("reels")?.unwrap();
    }
    info!("{id}: finished scraping reels");
    Ok(id.to_string())
}
