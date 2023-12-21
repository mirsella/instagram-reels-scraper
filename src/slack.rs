use std::path::Path;

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
        todo!()
    }
}
