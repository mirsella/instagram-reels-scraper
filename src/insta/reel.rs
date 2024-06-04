use chrono::{DateTime, Local};
use ordered_float::OrderedFloat;
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Serialize, Eq, PartialEq, Hash, Default, Clone)]
pub struct Reel {
    pub url: String,
    pub ratio: Option<OrderedFloat<f32>>,
    pub account: String,
    pub like: usize,
    pub comments: usize,
    pub views: Option<usize>,
    // #[serde(serialize_with = "serialize_custom_duration")]
    pub duration: Duration,
    // #[serde(serialize_with = "serialize_custom_date")]
    pub date: DateTime<Local>,
    pub paid_partnership: bool,
    pub caption: String,
}

impl From<&Value> for Reel {
    fn from(value: &Value) -> Self {
        let reel = &value.as_object().expect("a object");
        let code = reel["code"].as_str().unwrap();
        let like = reel["like_count"].as_u64().unwrap() as usize;
        let comments = reel["comment_count"].as_u64().unwrap() as usize;
        let views = reel["play_count"].as_u64().map(|v| v as usize);
        let caption = reel["caption"]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let duration = Duration::from_secs_f64(reel["video_duration"].as_f64().unwrap());
        let epoch_time_s = reel["taken_at"].as_i64().unwrap();
        let date = DateTime::from_timestamp(epoch_time_s, 0)
            .unwrap_or_default()
            .into();
        let account = reel["user"]["username"].as_str().unwrap().to_string();
        let paid_partnership = reel["is_paid_partnership"].as_bool().unwrap();
        Self {
            url: format!("https://www.instagram.com/reel/{code}/"),
            views,
            like,
            comments,
            date,
            duration,
            account,
            caption,
            paid_partnership,
            ratio: None,
        }
    }
}
impl Reel {
    pub fn set_ratio(&mut self, followers_count: usize) {
        let ratio = self.views.unwrap_or_default() as f32 / followers_count as f32;
        self.ratio = Some(OrderedFloat(ratio))
    }
}

fn _serialize_custom_date<S: Serializer>(
    date: &DateTime<Local>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let formatted_date = date.format("%Y-%m-%d %H:%M:%S").to_string();
    serializer.serialize_str(&formatted_date)
}

fn _serialize_custom_duration<S: Serializer>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let s = format!("{:?}", duration);
    serializer.serialize_str(&s)
}
