pub use crate::asset_context::{ArticleState, NativeYoutubeState, PdfState, YoutubeState};
use anyhow::Error;
use eur_proto::ipc::{SnapshotResponse, StateResponse};

pub struct JSONToProtoSnapshotConverter;

impl JSONToProtoSnapshotConverter {
    pub fn convert(object: &serde_json::Value) -> Result<SnapshotResponse, Error> {
        let json =
            serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(object.clone())?;

        eprintln!("JSONToProtoConverter::convert json: {:?}", json);

        // If success is false, return an error
        if !json.get("success").unwrap().as_bool().unwrap() {
            eprintln!("Failed to convert JSON to Proto, response: {:?}", json);
            return Err(anyhow::anyhow!("Failed to convert JSON to Proto"));
        }

        match json
            .get("type")
            .unwrap_or(&serde_json::Value::String("ARTICLE_STATE".to_string()))
            .as_str()
            .unwrap()
        {
            "YOUTUBE_SNAPSHOT" => {
                let native_state = NativeYoutubeState::from(&json);
                let proto_state = YoutubeState::from(&native_state);
                let state = eur_proto::ipc::snapshot_response::Snapshot::Youtube(proto_state.0);
                Ok(SnapshotResponse {
                    snapshot: Some(state),
                })
            }
            _ => Err(anyhow::anyhow!("Unsupported type")),
        }
    }
}
