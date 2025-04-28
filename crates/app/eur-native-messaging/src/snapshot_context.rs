use base64::prelude::*;
pub use eur_proto::ipc::ProtoYoutubeSnapshot;
pub use eur_proto::native_messaging::ProtoNativeYoutubeSnapshot;
pub use eur_proto::shared::ProtoImage;

// Wrapper type for ProtoYoutubeSnapshot
pub struct YoutubeSnapshot(pub ProtoYoutubeSnapshot);

// Wrapper type for ProtoNativeYoutubeSnapshot
pub struct NativeYoutubeSnapshot(pub ProtoNativeYoutubeSnapshot);

impl From<&serde_json::Map<String, serde_json::Value>> for NativeYoutubeSnapshot {
    fn from(obj: &serde_json::Map<String, serde_json::Value>) -> Self {
        eprintln!("NativeYoutubeSnapshot::from obj: {:?}", obj);
        NativeYoutubeSnapshot(ProtoNativeYoutubeSnapshot {
            r#type: obj.get("type").unwrap().as_str().unwrap().to_string(),
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

impl From<&NativeYoutubeSnapshot> for YoutubeSnapshot {
    fn from(obj: &NativeYoutubeSnapshot) -> Self {
        let video_frame_data = BASE64_STANDARD
            .decode(obj.0.video_frame_base64.as_str())
            .unwrap();

        YoutubeSnapshot(ProtoYoutubeSnapshot {
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
