use crate::config::{Config, USER_AGENT};
use anyhow::Result;
use headless_chrome::{Browser, LaunchOptionsBuilder, Tab};
use log::{debug, trace};
use std::{thread::sleep, time::Duration};

pub fn login(tab: &Tab, user: &str, pass: &str) -> Result<()> {
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
    trace!("save info");
    tab.find_element("button[type=button]")?.click()?;
    tab.wait_until_navigated()?;
    Ok(())
}

pub fn setup(config: &Config) -> Result<()> {
    trace!("launching browser");
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .user_data_dir(Some(config.chromedata.clone()))
            .headless(config.headless)
            .build()?,
    )?;
    let tab = browser.new_tab()?;
    tab.set_user_agent(USER_AGENT, None, None)?;
    tab.navigate_to("https://www.instagram.com/")?
        .wait_until_navigated()?;
    login(&tab, &config.insta_user, &config.insta_pass)?;
    Ok(())
}
