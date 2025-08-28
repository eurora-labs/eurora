use anyhow::Error;
use eur_proto::ipc::StateResponse;
use tracing::info;

pub use crate::asset_context::{
    ArticleState, NativeArticleAsset, NativeTwitterState, NativeYoutubeState, PdfState,
    TwitterState, YoutubeState,
};

pub struct JSONToProtoAssetConverter;

impl JSONToProtoAssetConverter {
    pub fn convert(object: &serde_json::Value) -> Result<StateResponse, Error> {
        let json =
            serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(object.clone())?;

        info!("JSONToProtoConverter::convert json: {:?}", json);

        // Check for success field and provide detailed error context
        let success = json
            .get("success")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| {
                anyhow::anyhow!("Missing or invalid 'success' field in JSON response")
            })?;

        if !success {
            let error_msg = json
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            info!(
                "Asset conversion failed - success: false, error: {}, full response: {:?}",
                error_msg, json
            );
            return Err(anyhow::anyhow!("Asset conversion failed: {}", error_msg));
        }

        match json
            .get("type")
            .unwrap_or(&serde_json::Value::String("ARTICLE_STATE".to_string()))
            .as_str()
            .unwrap()
        {
            "YOUTUBE_STATE" => {
                let native_state = NativeYoutubeState::from(&json);
                let proto_state = YoutubeState::try_from(&native_state)
                    .map_err(|e| anyhow::anyhow!("Failed to convert YouTube state: {}", e))?;
                let state = eur_proto::ipc::state_response::State::Youtube(proto_state.0);
                Ok(StateResponse { state: Some(state) })
            }
            "ARTICLE_ASSET" => {
                let native_state = NativeArticleAsset::from(&json);
                let proto_state = ArticleState::from(&native_state);
                let state = eur_proto::ipc::state_response::State::Article(proto_state.0);
                Ok(StateResponse { state: Some(state) })
            }
            "PDF_STATE" => {
                let proto_state = PdfState::from(&json);
                // We need to update the StateResponse union to include PDF state
                // Let's update the tauri_ipc.proto file to include PDF in the state oneof
                let state = eur_proto::ipc::state_response::State::Pdf(proto_state.0);
                Ok(StateResponse { state: Some(state) })
            }
            "TWITTER_STATE" => {
                let native_state = NativeTwitterState::from(&json);
                let proto_state = TwitterState::try_from(&native_state)
                    .map_err(|e| anyhow::anyhow!("Failed to convert Twitter state: {}", e))?;
                let state = eur_proto::ipc::state_response::State::Twitter(proto_state.0);
                Ok(StateResponse { state: Some(state) })
            }
            unknown_type => {
                info!(
                    "Unsupported asset type '{}' in JSON: {:?}",
                    unknown_type, json
                );
                Err(anyhow::anyhow!(
                    "Unsupported asset type: '{}'. Supported types: YOUTUBE_STATE, ARTICLE_ASSET, PDF_STATE, TWITTER_STATE",
                    unknown_type
                ))
            }
        }
    }
}
