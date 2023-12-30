use std::{ffi::OsStr, process::Command, time::Duration};

use anyhow::Context;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use log::debug;
use tempfile::TempDir;

use crate::config::Config;

pub struct BrowserWithTmpDir {
    browser: Browser,
    #[allow(dead_code)]
    tmpdir: Option<TempDir>,
}

impl std::ops::Deref for BrowserWithTmpDir {
    type Target = Browser;
    fn deref(&self) -> &Self::Target {
        &self.browser
    }
}
impl BrowserWithTmpDir {
    pub fn new(config: &Config, copy: bool) -> anyhow::Result<BrowserWithTmpDir> {
        debug!(
            "new browser copy: {copy} and data path {}",
            config.chromedata.display()
        );
        let tmpdir = match copy {
            true => {
                let tmpdir = tempfile::tempdir().context("creating tempdir")?;
                let status = Command::new("cp")
                    .arg("-r")
                    .arg(format!("{}/Default", config.chromedata.to_str().unwrap()))
                    .arg(tmpdir.path())
                    .status()
                    .expect("failed to execute cp -r");
                if !status.success() {
                    return Err(anyhow::anyhow!("cp -r failed"));
                }
                Some(tmpdir)
            }
            false => None,
        };
        let datadir = match &tmpdir {
            Some(tmpdir) => tmpdir.path().into(),
            None => config.chromedata.clone(),
        };
        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .user_data_dir(Some(datadir))
                .args(vec![OsStr::new("--blink-settings=imagesEnabled=false")])
                .idle_browser_timeout(Duration::from_secs(60))
                .headless(config.headless)
                .window_size(Some((1920, 1080)))
                .sandbox(false)
                .build()?,
        )
        .unwrap();
        Ok(Self { browser, tmpdir })
    }
}
