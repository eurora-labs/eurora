use anyhow::Result;
pub use eur_proto::{
    ipc::{
        ProtoArticleState, ProtoPdfState, ProtoTranscriptLine, ProtoTweet, ProtoTwitterState,
        ProtoYoutubeState,
    },
    native_messaging::{ProtoNativeArticleAsset, ProtoNativeTwitterState, ProtoNativeYoutubeState},
};
use serde::Deserialize;
use tracing::info;

#[derive(Deserialize)]
struct TranscriptLine {
    text: String,
    start: f32,
    duration: f32,
}

#[derive(Deserialize)]
struct TwitterTweet {
    text: String,
    timestamp: Option<String>,
    author: Option<String>,
}

impl From<TranscriptLine> for ProtoTranscriptLine {
    fn from(line: TranscriptLine) -> Self {
        ProtoTranscriptLine {
            text: line.text,
            start: line.start,
            duration: line.duration,
        }
    }
}

impl From<TwitterTweet> for ProtoTweet {
    fn from(tweet: TwitterTweet) -> Self {
        ProtoTweet {
            text: tweet.text,
            timestamp: tweet.timestamp,
            author: tweet.author,
        }
    }
}

// New wrapper type for ProtoYoutubeState
pub struct YoutubeState(pub ProtoYoutubeState);

pub struct NativeYoutubeState(pub ProtoNativeYoutubeState);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeYoutubeState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("NativeYoutubeState::from obj: {:?}", obj);
        NativeYoutubeState(ProtoNativeYoutubeState {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            url: obj.get("url").unwrap().as_str().unwrap().to_string(),
            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            transcript: obj.get("transcript").unwrap().as_str().unwrap().to_string(),
            current_time: obj.get("currentTime").unwrap().as_f64().unwrap() as f32,
        })
    }
}

impl TryFrom<&NativeYoutubeState> for YoutubeState {
    type Error = anyhow::Error;

    fn try_from(obj: &NativeYoutubeState) -> Result<Self> {
        // Parse the transcript string into Vec<TranscriptLine> and convert to Vec<ProtoTranscriptLine>
        let transcript = serde_json::from_str::<Vec<TranscriptLine>>(obj.0.transcript.as_str())
            .map(|lines| lines.into_iter().map(Into::into).collect())
            .unwrap_or_else(|_| Vec::new());

        Ok(YoutubeState(ProtoYoutubeState {
            url: obj.0.url.clone(),
            title: obj.0.title.clone(),
            transcript,
            current_time: obj.0.current_time,
        }))
    }
}

pub struct NativeArticleAsset(pub ProtoNativeArticleAsset);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeArticleAsset {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        NativeArticleAsset(ProtoNativeArticleAsset {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            content: obj.get("content").unwrap().as_str().unwrap().to_string(),
            text_content: obj
                .get("textContent")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            selected_text: obj
                .get("selectedText")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),

            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            site_name: obj.get("siteName").unwrap().as_str().unwrap().to_string(),

            language: obj.get("language").unwrap().as_str().unwrap().to_string(),
            excerpt: obj.get("excerpt").unwrap().as_str().unwrap().to_string(),
            length: obj.get("length").unwrap().as_i64().unwrap() as i32,
        })
    }
}

// New wrapper type for ProtoArticleState
pub struct ArticleState(pub ProtoArticleState);

impl From<&NativeArticleAsset> for ArticleState {
    fn from(obj: &NativeArticleAsset) -> Self {
        ArticleState(ProtoArticleState {
            content: obj.0.content.clone(),
            text_content: obj.0.text_content.clone(),
            selected_text: obj.0.selected_text.clone(),
            title: obj.0.title.clone(),
            site_name: obj.0.site_name.clone(),
            language: obj.0.language.clone(),
            excerpt: obj.0.excerpt.clone(),
            length: obj.0.length,
        })
    }
}

// impl From<&serde_json::Map<String, serde_json::Value>> for ArticleState {
//     fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
//         ArticleState(ProtoArticleState {
//             url: obj.get("url").unwrap().as_str().unwrap().to_string(),
//             title: obj.get("title").unwrap().as_str().unwrap().to_string(),
//             content: obj.get("content").unwrap().as_str().unwrap().to_string(),
//             selected_text: obj
//                 .get("selectedText")
//                 .unwrap()
//                 .as_str()
//                 .unwrap()
//                 .to_string(),
//         })
//     }
// }

// New wrapper type for ProtoPDFState
pub struct PdfState(pub ProtoPdfState);

impl From<&serde_json::Map<String, serde_json::Value>> for PdfState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("PdfState::from obj: {:?}", obj);
        PdfState(ProtoPdfState {
            url: obj.get("url").unwrap().as_str().unwrap().to_string(),
            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            content: obj.get("content").unwrap().as_str().unwrap().to_string(),
            selected_text: obj
                .get("selectedText")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
        })
    }
}

// New wrapper type for ProtoTwitterState
pub struct TwitterState(pub ProtoTwitterState);

pub struct NativeTwitterState(pub ProtoNativeTwitterState);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeTwitterState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        info!("NativeTwitterState::from obj: {:?}", obj);
        NativeTwitterState(ProtoNativeTwitterState {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            url: obj.get("url").unwrap().as_str().unwrap().to_string(),
            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            tweets: obj.get("tweets").unwrap().as_str().unwrap().to_string(),
            timestamp: obj.get("timestamp").unwrap().as_str().unwrap().to_string(),
        })
    }
}

impl TryFrom<&NativeTwitterState> for TwitterState {
    type Error = anyhow::Error;

    fn try_from(obj: &NativeTwitterState) -> Result<Self> {
        // Parse the tweets string into Vec<TwitterTweet> and convert to Vec<ProtoTweet>
        let tweets = serde_json::from_str::<Vec<TwitterTweet>>(obj.0.tweets.as_str())
            .map(|tweets| tweets.into_iter().map(Into::into).collect())
            .unwrap_or_else(|_| Vec::new());

        Ok(TwitterState(ProtoTwitterState {
            url: obj.0.url.clone(),
            title: obj.0.title.clone(),
            tweets,
            timestamp: obj.0.timestamp.clone(),
        }))
    }
}
