use crate::asset_context::NativeYoutubeState; // Only need NativeYoutubeState
use anyhow::{Error, anyhow}; // Import anyhow macro and Error
use base64::prelude::*; // Import base64
use eur_proto::ipc::{ProtoYoutubeSnapshot, SnapshotResponse}; // Import necessary proto types
use eur_proto::shared::ProtoImage; // Import ProtoImage

pub struct JSONToProtoSnapshotConverter;

impl JSONToProtoSnapshotConverter {
    pub fn convert(object: &serde_json::Value) -> Result<SnapshotResponse, Error> {
        let json =
            serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(object.clone())?;

        eprintln!("JSONToProtoSnapshotConverter::convert json: {:?}", json); // Corrected eprintln message

        // Check for success field, common in native messaging responses
        if let Some(success) = json.get("success") {
            if !success.as_bool().unwrap_or(false) {
                eprintln!("Snapshot generation failed in extension: {:?}", json);
                return Err(anyhow!("Snapshot generation failed in extension"));
            }
        } else {
            eprintln!("Missing 'success' field in snapshot response: {:?}", json);
            return Err(anyhow!("Missing 'success' field in snapshot response"));
        }

        // Determine the type of snapshot
        let snapshot_type = json
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'type' field in snapshot response"))?;

        match snapshot_type {
            "YOUTUBE_SNAPSHOT" => {
                let native_state = NativeYoutubeState::from(&json);

                // Decode the base64 video frame
                let video_frame_data = BASE64_STANDARD
                    .decode(native_state.0.video_frame_base64.as_str())
                    .map_err(|e| anyhow!("Failed to decode base64 video frame: {}", e))?;

                // Construct ProtoYoutubeSnapshot directly
                let proto_snapshot = ProtoYoutubeSnapshot {
                    current_time: native_state.0.current_time,
                    video_frame: Some(ProtoImage {
                        data: video_frame_data,
                        width: native_state.0.video_frame_width,
                        height: native_state.0.video_frame_height,
                        format: native_state.0.video_frame_format,
                    }),
                };

                let snapshot_field =
                    eur_proto::ipc::snapshot_response::Snapshot::Youtube(proto_snapshot);
                Ok(SnapshotResponse {
                    snapshot: Some(snapshot_field),
                })
            }
            // Add cases for other snapshot types here if needed in the future
            // e.g., "ARTICLE_SNAPSHOT", "PDF_SNAPSHOT"
            _ => Err(anyhow!("Unsupported snapshot type: {}", snapshot_type)),
        }
    }
}
