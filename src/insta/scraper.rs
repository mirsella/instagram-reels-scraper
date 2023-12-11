use std::sync::{mpsc::Sender, Arc, Mutex};

use headless_chrome::Tab;

use super::Reel;

pub fn scraper(
    tab: Arc<Tab>,
    accounts: Arc<Mutex<Vec<String>>>,
    tx: Sender<Reel>,
) -> anyhow::Result<()> {
    let account = match accounts.lock().unwrap().pop() {
        Some(account) => account,
        None => return Ok(()),
    };
    tx.send(Reel {
        id: "xyz".into(),
        caption: account,
        ..Default::default()
    })?;
    Ok(())
}

