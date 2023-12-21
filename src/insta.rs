mod scraper;
use crate::{
    browserwithtmpdir::BrowserWithTmpDir,
    config::{Config, USER_AGENT},
};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use core::fmt;
use futures::executor::block_on;
use log::{debug, error, info, trace};
use scraper::scraper;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Reel {
    #[serde(skip_serializing)]
    pub id: String,
    pub link: String,
    pub account: String,
    pub caption: String,
    pub like: usize,
    pub comments: usize,
    pub views: Option<usize>,
    pub duration: usize,
    #[serde(skip_serializing)]
    pub date: DateTime<Utc>,
}
impl From<&Value> for Reel {
    fn from(value: &Value) -> Self {
        let reel = &value["media"];
        let caption = reel
            .get("caption")
            .map(|v: &Value| v["text"].clone())
            .unwrap_or_default();
        let id = reel["code"].as_str().unwrap().into();
        let views = reel["play_count"]
            .as_u64()
            .or_else(|| reel["view_count"].as_u64())
            .map(|v| v as usize);
        let like = reel["like_count"].as_u64().unwrap() as usize;
        let comments = reel["comment_count"].as_u64().unwrap() as usize;
        let duration = reel["video_duration"].as_f64().unwrap() as usize;
        let epoch_time = reel["device_timestamp"].as_f64().unwrap() as usize;
        let date = DateTime::from_timestamp(epoch_time as i64, 0).unwrap_or_else(|| {
            DateTime::from_timestamp((epoch_time / 1_000_000) as i64, 0)
                .unwrap_or_else(|| panic!("invalid epoch time: {}", epoch_time))
        });
        // println!("\n{:#}\n", reel);
        let account = reel["user"]["username"].as_str().unwrap().to_string();
        Self {
            caption: caption.as_str().unwrap_or("no caption").into(),
            link: format!("https://www.instagram.com/reel/{}/", id),
            id,
            views,
            like,
            comments,
            duration,
            date,
            account,
        }
    }
}
impl fmt::Display for Reel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}: {:.40}, {} likes, {} comments, {:#?} views, {} seconds",
            self.account,
            self.id,
            self.caption,
            self.like,
            self.comments,
            self.views,
            self.duration,
            // self.date
        )
    }
}

fn login(config: &Config) -> Result<()> {
    info!("login");
    let browser = BrowserWithTmpDir::new(config, false).context("new browser")?;
    let tab = browser.new_tab()?;
    tab.enable_stealth_mode()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    tab.navigate_to("https://www.instagram.com/accounts/login/")?
        .wait_until_navigated()?;
    if let Ok(el) = tab.find_element_by_xpath(
        "//button[contains(text(), 'Allow all cookies') or contains(text(), 'Accepter')]",
    ) {
        trace!("accepting cookies");
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
    info!("logged in");
    Ok(())
}

pub fn run(config: &Config) -> Result<(Receiver<Reel>, JoinHandle<()>)> {
    login(config)?;
    let urls = Arc::new(Mutex::new(Vec::from_iter(config.accounts.clone())));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(config.worker);
    trace!("starting {} workers", config.worker);
    for i in 0..config.worker {
        let browser = BrowserWithTmpDir::new(config, true)?;
        let tx = tx.clone();
        let urls = urls.clone();
        handles.push(
            thread::Builder::new()
                .name(i.to_string())
                .spawn(move || scraper(browser, urls, tx))
                .unwrap(),
        );
    }
    let telegram = rustygram::create_bot(&config.telegram_token, &config.telegram_chat_id);
    let handle = thread::spawn(move || {
        trace!("waiting for {} workers to finish", handles.len());
        while !handles.is_empty() {
            if let Some(pos) = handles.iter().position(|h| h.is_finished()) {
                let handle = handles.remove(pos);
                let name = handle
                    .thread()
                    .name()
                    .map(ToString::to_string)
                    .unwrap_or(format!("id:{:?}", thread::current().id()));
                match handle.join() {
                    Ok(Err(e)) => {
                        error!("worker {name} thread error: {e:?}");
                        if let Err(te) = block_on(telegram.send_message(
                            &format!("instagram-reels-scraper: worker {name} thread error: {e:?}"),
                            None,
                        )) {
                            error!("telegram error: {te:?}");
                        }
                    }
                    Err(e) => {
                        error!("worker {name} thread panicked: {e:?}");
                        if let Err(te) = block_on(telegram.send_message(
                            &format!(
                                "instagram-reels-scraper: worker {name} thread panicked: {e:?}"
                            ),
                            None,
                        )) {
                            error!("telegram error: {te:?}");
                        }
                    }
                    Ok(Ok(id)) => info!("{name}({id}) worker thread finished"),
                }
            }
            sleep(Duration::from_millis(10));
        }
        info!("all worker threads finished");
    });
    Ok((rx, handle))
}
