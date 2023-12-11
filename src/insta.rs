mod scraper;
use crate::config::{Config, USER_AGENT};
use anyhow::Result;
use core::fmt;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use log::{debug, error, trace};
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep},
    time::Duration,
};

use self::scraper::scraper;

#[derive(Debug, Default)]
pub struct Reel {
    pub id: String,
    pub caption: String,
    pub like: usize,
    pub comments: usize,
    pub views: usize,
}
impl fmt::Display for Reel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Reel {}, caption: {:.40}, {} likes, {} comments, {} views",
            self.id, self.caption, self.like, self.comments, self.views
        )
    }
}

fn login(tab: &Tab, user: &str, pass: &str) -> Result<()> {
    debug!("logging in");
    tab.find_element("input[name=username]")?.type_into(user)?;
    tab.find_element("input[name=password]")?.type_into(pass)?;
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
    Ok(())
}

fn setup(config: &Config) -> Result<Browser> {
    trace!("launching browser");
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .user_data_dir(Some(config.chromedata.clone()))
            .headless(config.headless)
            .build()?,
    )?;
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
        login(&tab, &config.insta_user, &config.insta_pass)?;
    }
    Ok(browser)
}

pub fn run(config: &Config) -> Result<Receiver<Reel>> {
    let browser = setup(config)?;
    debug!("setup finished");
    let urls = Arc::new(Mutex::new(Vec::from_iter(config.accounts.clone())));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(config.worker);
    trace!("starting {} workers", config.worker);
    for _ in 0..config.worker {
        let context = browser.new_context()?;
        let tab = context.new_tab()?;
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
