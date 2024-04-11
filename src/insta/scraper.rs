use super::Reel;
use crate::{browserwithtmpdir::BrowserWithTmpDir, config::USER_AGENT};
use anyhow::Context;
use chrono::NaiveTime;
use headless_chrome::browser::tab::ResponseHandler;
use log::{debug, info};
use std::{
    sync::{
        mpsc::{self, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

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
    let tab = browser.new_tab()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    let thread = thread::current();
    let id: String = thread
        .name()
        .map(ToString::to_string)
        .unwrap_or(format!("{:?}", thread.id()));

    let (tx, rx) = mpsc::channel::<Reel>();
    let mut handler: ResponseHandler = Box::new(move |res, fetch_body| {
        if !res.response.url.ends_with("/info/") || !res.response.url.contains("/api/v1/media/") {
            return;
        }
        let body = wait_for_body(fetch_body).body;
        let body: serde_json::Value = serde_json::from_str(&body).expect("valid json");
        let reel = &body["items"][0];
        tx.send(reel.into()).unwrap();
    });

    let month_ago = chrono::Local::now()
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap()
        - chrono::Duration::days(30);
    loop {
        let account = match accounts.lock().unwrap().pop() {
            Some(account) => account,
            None => break,
        };
        info!("{id}: scraping reels of {account}");
        tab.register_response_handling("reels", handler)?;
        tab.navigate_to(&format!("https://www.instagram.com/{account}/reels/"))
            .context("navigate_to")?;
        tab.wait_until_navigated()?;

        let followers = tab
            .wait_for_element("a[href$='/followers/'] > span")
            .context("followers element")?
            .get_attribute_value("title")
            .unwrap()
            .context("no title attribute")?
            .replace(',', "")
            .parse()
            .context("parsing followers count")?;
        debug!("{id}: got {followers} followers for {account}. starting scrolling");

        tab.find_element("a[href^='/reel/']")
            .context("finding first reel")?
            .click()
            .expect("clicking");

        let mut count = 0;
        while let Ok(mut reel) = rx.recv() {
            if reel.date < month_ago {
                break;
            }
            tab.evaluate(
                "document.querySelector(\"svg[aria-label='Next']\").parentElement.parentElement.parentElement.click()",
                false,
            ).context("clicking on the next reel")?;
            reel.set_ratio(followers);
            count += 1;
            main_tx.send(reel)?;
        }
        info!("{id}: got {count} reels on {account}");
        handler = tab.deregister_response_handling("reels")?.unwrap();
    }
    info!("{id}: finished scraping reels");
    Ok(id.to_string())
}
