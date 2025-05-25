use crate::snapshot_context::{NativeYoutubeSnapshot, YoutubeSnapshot}; // Use snapshot context types
use anyhow::{anyhow, Error}; // Import anyhow macro and Error
use eur_proto::ipc::SnapshotResponse; // Import necessary proto types

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
            // Add cases for other snapshot types here if needed in the future
            // e.g., "ARTICLE_SNAPSHOT", "PDF_SNAPSHOT"
            _ => Err(anyhow!("Unsupported snapshot type: {}", snapshot_type)),
        }
    }
}
