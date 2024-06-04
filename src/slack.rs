use anyhow::anyhow;
use log::info;
use serde_json::Value;
use std::path::Path;
use ureq_multipart::MultipartBuilder;

#[deprecated = "The files.upload endpoint it deprecated"]
pub struct SlackFileSender {
    token: String,
    channel: String,
}
#[allow(deprecated)]
impl SlackFileSender {
    pub fn new(token: impl Into<String>, channel: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            channel: channel.into(),
        }
    }
    pub fn send_file(&self, path: &Path) -> anyhow::Result<Value> {
        let (content_type, data) = MultipartBuilder::new()
            .add_file("file", path)?
            .add_text(
                "filetype",
                path.extension()
                    .ok_or(anyhow!("not extension for this filetype"))?
                    .to_str()
                    .unwrap(),
            )?
            .add_text("channels", &self.channel)?
            .add_text("token", &self.token)?
            .finish()?;
        info!("sending {path:?} to slack");
        let response = ureq::post("https://slack.com/api/files.upload")
            .set("Content-Type", &content_type)
            .send_bytes(&data)?;
        let json: Value = response.into_json()?;
        json.get("ok")
            .and_then(Value::as_bool)
            .ok_or_else(|| anyhow!("field `ok` not found in response: {json:#}"))?;
        info!("sent {path:?} to slack");
        Ok(json)
    }
}
