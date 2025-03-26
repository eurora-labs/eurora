use base64::prelude::*;
pub use eur_proto::ipc::{
    ProtoArticleState, ProtoPdfState, ProtoTranscriptLine, ProtoYoutubeState,
};
pub use eur_proto::native_messaging::ProtoNativeYoutubeState;
pub use eur_proto::shared::{ProtoImage, ProtoImageFormat};
use serde::Deserialize;

#[derive(Deserialize)]
struct TranscriptLine {
    text: String,
    start: f32,
    duration: f32,
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

// New wrapper type for ProtoYoutubeState
pub struct YoutubeState(pub ProtoYoutubeState);

pub struct NativeYoutubeState(pub ProtoNativeYoutubeState);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeYoutubeState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        eprintln!("NativeYoutubeState::from obj: {:?}", obj);
        NativeYoutubeState(ProtoNativeYoutubeState {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
            url: obj.get("url").unwrap().as_str().unwrap().to_string(),
            title: obj.get("title").unwrap().as_str().unwrap().to_string(),
            transcript: obj.get("transcript").unwrap().as_str().unwrap().to_string(),
            current_time: obj.get("currentTime").unwrap().as_f64().unwrap() as f32,
            video_frame_base64: obj
                .get("videoFrameBase64")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string(),
            video_frame_width: obj.get("videoFrameWidth").unwrap().as_i64().unwrap() as i32,
            video_frame_height: obj.get("videoFrameHeight").unwrap().as_i64().unwrap() as i32,
            video_frame_format: obj.get("videoFrameFormat").unwrap().as_i64().unwrap() as i32,
        })
    }
}

impl From<&NativeYoutubeState> for YoutubeState {
    fn from(obj: &NativeYoutubeState) -> Self {
        let video_frame_data = BASE64_STANDARD
            .decode(obj.0.video_frame_base64.as_str())
            .unwrap();

        // Parse the transcript string into Vec<TranscriptLine> and convert to Vec<ProtoTranscriptLine>
        let transcript = serde_json::from_str::<Vec<TranscriptLine>>(obj.0.transcript.as_str())
            .map(|lines| lines.into_iter().map(Into::into).collect())
            .unwrap_or_else(|_| Vec::new());

        YoutubeState(ProtoYoutubeState {
            url: obj.0.url.clone(),
            title: obj.0.title.clone(),
            transcript,
            current_time: obj.0.current_time,
            video_frame: Some(ProtoImage {
                data: video_frame_data,
                width: obj.0.video_frame_width,
                height: obj.0.video_frame_height,
                format: obj.0.video_frame_format,
            }),
        })
    }
}

// impl From<&serde_json::Map<String, serde_json::Value>> for YoutubeState {
//     fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
//         // Convert the video frame from base64 to a Vec<u8>
//         let video_frame = BASE64_STANDARD
//             .decode(obj.get("videoFrameBase64").unwrap().as_str().unwrap())
//             .unwrap();
//         YoutubeState(ProtoYoutubeState {
//             url: obj.get("url").unwrap().as_str().unwrap().to_string(),
//             title: obj.get("title").unwrap().as_str().unwrap().to_string(),
//             transcript: serde_json::from_str::<Vec<TranscriptLine>>(
//                 obj.get("transcript").unwrap().as_str().unwrap(),
//             )
//             .map(|lines| lines.into_iter().map(Into::into).collect())
//             .unwrap_or_else(|_| Vec::new()),
//             current_time: obj.get("currentTime").unwrap().as_f64().unwrap() as f32,
//             video_frame: Some(ProtoImage {
//                 data: video_frame,
//                 width: 0,
//                 height: 0,
//                 format: ProtoImageFormat::Jpeg as i32,
//             }),
//         })
//     }
// }

// New wrapper type for ProtoArticleState
pub struct ArticleState(pub ProtoArticleState);

impl From<&serde_json::Map<String, serde_json::Value>> for ArticleState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        ArticleState(ProtoArticleState {
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

// New wrapper type for ProtoPDFState
pub struct PdfState(pub ProtoPdfState);

impl From<&serde_json::Map<String, serde_json::Value>> for PdfState {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        eprintln!("PdfState::from obj: {:?}", obj);
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
