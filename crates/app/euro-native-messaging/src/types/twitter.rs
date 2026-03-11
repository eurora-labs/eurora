use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Default, Type)]
pub struct NativeTwitterTweet {
    pub text: String,
    pub timestamp: Option<String>,
    pub author: Option<String>,
    #[serde(default)]
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TweetPageData {
    pub tweet: Option<NativeTwitterTweet>,
    pub replies: Vec<NativeTwitterTweet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct ProfilePageData {
    pub username: String,
    pub tweets: Vec<NativeTwitterTweet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct TimelineData {
    pub tweets: Vec<NativeTwitterTweet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct SearchData {
    pub query: String,
    pub tweets: Vec<NativeTwitterTweet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NotificationsData {
    pub tweets: Vec<NativeTwitterTweet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct UnsupportedPageData {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(tag = "page", content = "data")]
pub enum ParseResult {
    #[serde(rename = "tweet")]
    Tweet(TweetPageData),
    #[serde(rename = "profile")]
    Profile(ProfilePageData),
    #[serde(rename = "home")]
    Home(TimelineData),
    #[serde(rename = "search")]
    Search(SearchData),
    #[serde(rename = "notifications")]
    Notifications(NotificationsData),
    #[serde(rename = "unsupported")]
    Unsupported(UnsupportedPageData),
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
pub struct NativeTwitterAsset {
    pub url: String,
    pub title: String,
    pub result: ParseResult,
    pub timestamp: String,
}
