use anyhow::{bail, Context};
use log::debug;
use log::info;
use serde_json::json;
use serde_json::Value;
use std::{fs, path::Path};
use ureq_multipart::MultipartBuilder;

pub struct SlackFileSender {
    token: String,
    channel: String,
}
impl SlackFileSender {
    pub fn new(token: impl Into<String>, channel: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            channel: channel.into(),
        }
    }
    pub fn send_file(&self, path: &Path) -> anyhow::Result<()> {
        let content = fs::read(path).context("reading file")?;

        let (content_type, data) = MultipartBuilder::new()
            .add_text("length", &content.len().to_string())?
            .add_text("filename", path.to_str().unwrap())?
            .add_text("token", &self.token)?
            .finish()?;
        let json: Value = ureq::post("https://slack.com/api/files.getUploadURLExternal")
            .content_type(&content_type)
            .send(&data)
            .context("http request")?
            .body_mut()
            .read_json()
            .context("parsing response")?;
        if !json["ok"].as_bool().unwrap_or_default() {
            bail!("non-ok response on getUploadURLExternal: {json:?}");
        }
        debug!("slack getUploadURLExternal done");

        let status = ureq::post(json["upload_url"].as_str().unwrap())
            .send(&content)
            .context("http request")?
            .status();
        if status.as_u16() != 200 {
            bail!("non-200 response on upload_url: {status}");
        }
        debug!("slack upload_url done");

        let file_id = json["file_id"].as_str().unwrap();
        let (content_type, data) = MultipartBuilder::new()
            .add_text(
                "files",
                &json!([{"id": file_id, "title": path}]).to_string(),
            )?
            .add_text("token", &self.token)?
            .add_text("channel_id", &self.channel)?
            .finish()?;
        let json: Value = ureq::post("https://slack.com/api/files.completeUploadExternal")
            .content_type(&content_type)
            .send(&data)
            .context("http request")?
            .body_mut()
            .read_json()
            .context("parsing response")?;
        if !json["ok"].as_bool().unwrap_or_default() {
            bail!("non-ok response on completeUploadExternal: {json:?}");
        }
        debug!("slack completeUploadExternal done");
        info!("sent {path:?} to slack");
        Ok(())
    }
}
