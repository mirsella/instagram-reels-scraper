use std::{fmt, time::Duration};

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Reel {
    #[serde(skip_serializing)]
    pub id: String,
    pub link: String,
    pub ratio: Option<f32>,
    pub account: String,
    pub like: usize,
    pub comments: usize,
    pub views: Option<usize>,
    #[serde(serialize_with = "serialize_custom_duration")]
    pub duration: Duration,
    #[serde(serialize_with = "serialize_custom_date")]
    pub date: DateTime<Local>,
    pub caption: String,
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
        let duration = reel["video_duration"].as_f64().unwrap();
        let epoch_time_s = reel["taken_at"].as_i64().unwrap();
        let date = DateTime::from_timestamp(epoch_time_s, 0)
            .unwrap_or_default()
            .into();
        let account = reel["user"]["username"].as_str().unwrap().to_string();
        Self {
            caption: caption.as_str().unwrap_or("no caption").into(),
            link: format!("https://www.instagram.com/reel/{}/", id),
            id,
            views,
            like,
            comments,
            duration: Duration::from_secs_f64(duration),
            date,
            account,
            ..Default::default()
        }
    }
}
impl fmt::Display for Reel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}: {:.40}, {} likes, {} comments, {:#?} views, {:?}",
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
impl Reel {
    pub fn set_ratio(&mut self, followers: usize) {
        self.ratio = Some(self.views.unwrap_or_default() as f32 / followers as f32)
    }
}

fn serialize_custom_date<S: Serializer>(
    date: &DateTime<Local>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let formatted_date = date.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted_date)
}

fn serialize_custom_duration<S: Serializer>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let s = format!("{:?}", duration);
    serializer.serialize_str(&s)
}
