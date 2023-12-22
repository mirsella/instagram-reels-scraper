use std::fmt;

use chrono::{DateTime, Utc};
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
    pub duration: usize,
    #[serde(serialize_with = "serialize_custom_date")]
    pub date: DateTime<Utc>,
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
        let duration = reel["video_duration"].as_f64().unwrap() as usize;
        let epoch_time_unknown = reel["device_timestamp"].as_u64().unwrap();
        let epoch_time_s: usize = format!("{:0<.10}", epoch_time_unknown.to_string())
            .parse()
            .unwrap();
        let date = DateTime::from_timestamp(epoch_time_s as i64, 0).unwrap_or_default();
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
            ..Default::default()
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
impl Reel {
    pub fn set_ratio(&mut self, followers: usize) {
        self.ratio = Some(self.views.unwrap_or_default() as f32 / followers as f32)
    }
}

fn serialize_custom_date<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted_date = date.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted_date)
}
