pub use crate::asset_context::{
    ArticleState, NativeArticleAsset, NativeYoutubeState, PdfState, YoutubeState,
};
use anyhow::Error;
use eur_proto::ipc::StateResponse;

pub struct JSONToProtoAssetConverter;

impl JSONToProtoAssetConverter {
    pub fn convert(object: &serde_json::Value) -> Result<StateResponse, Error> {
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
            "YOUTUBE_STATE" => {
                let native_state = NativeYoutubeState::from(&json);
                let proto_state = YoutubeState::from(&native_state);
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
            _ => Err(anyhow::anyhow!("Unsupported type")),
        }
    }
}
