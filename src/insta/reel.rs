use chrono::{DateTime, Local};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::time::Duration;

#[derive(Debug, Default, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Reel {
    #[serde(skip_serializing)]
    pub id: String,
    pub link: String,
    pub ratio: Option<OrderedFloat<f32>>,
    pub account: Option<String>,
    pub like: usize,
    pub comments: usize,
    pub views: Option<usize>,
    // #[serde(serialize_with = "serialize_custom_duration")]
    // pub duration: Duration,
    // #[serde(serialize_with = "serialize_custom_date")]
    // pub date: DateTime<Local>,
    // pub caption: String,
}

// impl PartialEq for Reel {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

// impl hash::Hash for Reel {
//     fn hash<H: hash::Hasher>(&self, state: &mut H) {
//         self.id.hash(state);
//     }
// }

impl From<&Value> for Reel {
    fn from(value: &Value) -> Self {
        let reel = &value.as_object().expect("a object");
        let id = reel["code"].as_str().unwrap().into();
        let like = reel["like_count"].as_u64().unwrap() as usize;
        let comments = reel["comment_count"].as_u64().unwrap() as usize;
        let views = reel["play_count"].as_u64().map(|v| v as usize);
        Self {
            link: format!("https://www.instagram.com/reel/{}/", id),
            id,
            views,
            like,
            comments,
            ..Default::default()
        }
    }
}
impl Reel {
    pub fn set_followers(&mut self, followers: usize) {
        let ratio = self.views.unwrap_or_default() as f32 / followers as f32;
        self.ratio = Some(OrderedFloat(ratio))
    }

    pub fn set_account(&mut self, account: Option<String>) {
        self.account = account;
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
