use anyhow::{Error, anyhow}; // Import anyhow macro and Error
use eur_proto::ipc::SnapshotResponse; // Import necessary proto types
use tracing::info;

use crate::snapshot_context::{
    ArticleSnapshot, NativeArticleSnapshot, NativeTwitterSnapshot, NativeYoutubeSnapshot,
    TwitterSnapshot, YoutubeSnapshot,
}; // Use snapshot context types
pub struct JSONToProtoSnapshotConverter;

impl JSONToProtoSnapshotConverter {
    pub fn convert(object: &serde_json::Value) -> Result<SnapshotResponse, Error> {
        let json =
            serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(object.clone())?;

        info!("JSONToProtoSnapshotConverter::convert json: {:?}", json); // Corrected eprintln message

        // Check for success field, common in native messaging responses
        if let Some(success) = json.get("success") {
            if !success.as_bool().unwrap_or(false) {
                info!("Snapshot generation failed in extension: {:?}", json);
                return Err(anyhow!("Snapshot generation failed in extension"));
            }
        } else {
            info!("Missing 'success' field in snapshot response: {:?}", json);
            return Err(anyhow!("Missing 'success' field in snapshot response"));
        }

        // Determine the type of snapshot
        let snapshot_type = json
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("Missing or invalid 'type' field in snapshot response"))?;

        match snapshot_type {
            "YOUTUBE_SNAPSHOT" => {
                // Convert JSON to NativeYoutubeSnapshot
                let native_snapshot = NativeYoutubeSnapshot::from(&json);

                // Convert NativeYoutubeSnapshot to YoutubeSnapshot
                let youtube_snapshot = YoutubeSnapshot::try_from(&native_snapshot)
                    .map_err(|e| anyhow!("Failed to convert YouTube snapshot: {}", e))?;

                // Create the snapshot field for the response
                let snapshot_field =
                    eur_proto::ipc::snapshot_response::Snapshot::Youtube(youtube_snapshot.0);
                Ok(SnapshotResponse {
                    snapshot: Some(snapshot_field),
                })
            }
            "ARTICLE_SNAPSHOT" => {
                // Convert JSON to NativeArticleSnapshot
                let native_snapshot = NativeArticleSnapshot::from(&json);

                // Convert NativeArticleSnapshot to ArticleSnapshot
                let article_snapshot = ArticleSnapshot::try_from(&native_snapshot)
                    .map_err(|e| anyhow!("Failed to convert article snapshot: {}", e))?;

                // Create the snapshot field for the response
                let snapshot_field =
                    eur_proto::ipc::snapshot_response::Snapshot::Article(article_snapshot.0);
                Ok(SnapshotResponse {
                    snapshot: Some(snapshot_field),
                })
            }
            "TWITTER_SNAPSHOT" => {
                // Convert JSON to NativeTwitterSnapshot
                let native_snapshot = NativeTwitterSnapshot::from(&json);

                // Convert NativeTwitterSnapshot to TwitterSnapshot
                let twitter_snapshot = TwitterSnapshot::try_from(&native_snapshot)
                    .map_err(|e| anyhow!("Failed to convert Twitter snapshot: {}", e))?;

                // Create the snapshot field for the response
                let snapshot_field =
                    eur_proto::ipc::snapshot_response::Snapshot::Twitter(twitter_snapshot.0);
                Ok(SnapshotResponse {
                    snapshot: Some(snapshot_field),
                })
            }
            // Add cases for other snapshot types here if needed in the future
            // e.g., "PDF_SNAPSHOT"
            _ => Err(anyhow!("Unsupported snapshot type: {}", snapshot_type)),
        }
    }
}
