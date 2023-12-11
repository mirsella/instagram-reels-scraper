mod scraper;
use crate::config::{Config, USER_AGENT};
use anyhow::Result;
use core::fmt;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::{debug, error, trace};
use scraper::scraper;
use serde_json::Value;
use std::{
    ffi::OsStr,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::Duration,
};

#[derive(Debug, Default)]
pub struct Reel {
    pub id: String,
    pub caption: String,
    pub like: usize,
    pub comments: usize,
    pub views: usize,
    pub duration: usize,
}
impl From<&Value> for Reel {
    fn from(value: &Value) -> Self {
        let reel = &value["media"];
        let caption = reel
            .get("caption")
            .map(|v: &Value| v["text"].clone())
            .unwrap_or_default();
        let id = reel["code"].as_str().unwrap_or_default().into();
        let views = reel["play_count"].as_u64().unwrap_or_default() as usize;
        let like = reel["like_count"].as_u64().unwrap_or_default() as usize;
        let comments = reel["comment_count"].as_u64().unwrap_or_default() as usize;
        let duration = reel["video_duration"].as_u64().unwrap_or_default() as usize;
        Self {
            caption: caption.as_str().unwrap_or("no caption").into(),
            id,
            views,
            like,
            comments,
            duration,
        }
    }
}
impl fmt::Display for Reel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Reel {}:{:.40}, {} likes, {} comments, {} views, {}",
            self.id, self.caption, self.like, self.comments, self.views, self.duration
        )
    }
}

fn login(config: &Config) -> Result<()> {
    let browser = new_browser(config)?;
    let tab = browser.new_tab()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    tab.navigate_to("https://www.instagram.com/accounts/login/")?
        .wait_until_navigated()?;
    if let Ok(el) = tab.find_element_by_xpath(
        "//button[contains(text(), 'Allow all cookies') or contains(text(), 'Accepter')]",
    ) {
        debug!("accepting cookies");
        el.click()?;
        tab.wait_until_navigated()?;
        sleep(Duration::from_secs(2));
    }
    if tab.find_element("input[name=username]").is_ok() {
        debug!("logging in");
        tab.find_element("input[name=username]")?
            .type_into(&config.insta_user)?;
        tab.find_element("input[name=password]")?
            .type_into(&config.insta_pass)?;
        tab.find_element("button[type=submit]")?.click()?;
        trace!("wait for redirect");
        while tab
            .get_url()
            .starts_with("https://www.instagram.com/accounts/login")
        {
            sleep(Duration::from_millis(100));
        }
        tab.wait_until_navigated()?;
        trace!("finish waiting for redirect");
        if let Ok(el) = tab.find_element("button[type=button]") {
            trace!("save info");
            el.click()?;
        }
        tab.wait_until_navigated()?;
    }
    Ok(())
}

fn new_browser(config: &Config) -> Result<Browser> {
    trace!("launching browser");
    Browser::new(
        LaunchOptionsBuilder::default()
            .user_data_dir(Some(config.chromedata.clone()))
            .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
            .headless(config.headless)
            .build()?,
    )
}

pub fn run(config: &Config) -> Result<Receiver<Reel>> {
    login(config)?;
    let urls = Arc::new(Mutex::new(Vec::from_iter(config.accounts.clone())));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(config.worker);
    trace!("starting {} workers", config.worker);
    for _ in 0..config.worker {
        let browser = new_browser(config)?;
        let tab = browser.new_tab()?;
        tab.set_user_agent(USER_AGENT, None, None)?;
        let tx = tx.clone();
        let urls = urls.clone();
        handles.push(thread::spawn(move || scraper(tab, urls, tx)));
    }
    trace!("waiting for workers to finish");
    while !handles.is_empty() {
        if let Some(pos) = handles.iter().position(|h| h.is_finished()) {
            match handles.remove(pos).join() {
                Ok(Err(e)) => {
                    error!("worker thread error: {e:?}");
                    // TODO: telegram
                }
                Err(e) => {
                    error!("worker thread panicked: {e:?}");
                    // TODO: telegram
                }
                _ => (),
            }
        }
        sleep(Duration::from_millis(100));
    }
    Ok(rx)
}
