mod reel;
mod scraper;
use crate::{
    browserwithtmpdir::BrowserWithTmpDir,
    config::{Config, USER_AGENT},
    telegram::Telegram,
};
use anyhow::{Context, Result};
use log::{debug, error, info};
pub use reel::Reel;
use scraper::scraper;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread::{self, sleep, JoinHandle},
    time::Duration,
};

fn login(config: &Config) -> Result<()> {
    info!("login");
    let browser = BrowserWithTmpDir::new(config, false).context("new browser")?;
    let tab = browser.new_tab()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    tab.navigate_to("https://www.instagram.com/accounts/login/")?
        .wait_until_navigated()?;
    if let Ok(el) = tab.find_element_by_xpath(
        "//button[contains(text(), 'Decline') or contains(text(), 'Refuser')]",
    ) {
        debug!("declining cookies");
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
        debug!("wait for redirect");
        while tab
            .get_url()
            .starts_with("https://www.instagram.com/accounts/login")
        {
            sleep(Duration::from_millis(100));
        }
        tab.wait_until_navigated()?;
        thread::sleep(Duration::from_secs(2));
        debug!("finish waiting for redirect");
        if let Ok(el) = tab.find_element("button[type=button]") {
            debug!("save info");
            el.click()?;
            tab.wait_until_navigated()?;
            thread::sleep(Duration::from_secs(2));
        }
        tab.wait_until_navigated()?;
        thread::sleep(Duration::from_secs(2));
    }
    info!("logged in");
    Ok(())
}

pub fn run(config: &Config) -> Result<(Receiver<Reel>, JoinHandle<()>)> {
    login(config)?;
    let urls = Arc::new(Mutex::new(Vec::from_iter(config.accounts.clone())));
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::with_capacity(config.worker);
    debug!("starting {} workers", config.worker);
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
    let telegram = Telegram::new(&config.telegram_token, &config.telegram_chat_id);
    let handle = thread::spawn(move || {
        debug!("waiting for {} workers to finish", handles.len());
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
                        if let Err(te) = telegram.send(format!(
                            "instagram-reels-scraper: worker {name} thread error: {e:?}"
                        )) {
                            error!("telegram error: {te:?}");
                        }
                    }
                    Err(e) => {
                        error!("worker {name} thread panicked: {e:?}");
                        if let Err(te) = telegram.send(format!(
                            "instagram-reels-scraper: worker {name} thread panicked: {e:?}"
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
