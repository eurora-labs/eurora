use serde::{Deserialize, Serialize};
use specta::Type;

/// Snapshot of a Word document's textual content.
///
/// Sent by the Office add-in's runtime as the JSON payload of a
/// `ResponseFrame` for `GET_ASSETS`. The desktop deserializes this
/// directly — there is no `NativeMessage` discriminator wrapper because
/// the WebSocket transport (unlike Chrome's stdio native-messaging)
/// does not require a single envelope shape per message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WordDocumentAsset {
    /// Document title as reported by `Word.Document.properties.title`,
    /// or a fallback string when the document is unsaved/untitled.
    pub document_name: String,
    /// Full document body text (`Word.Document.body.text`).
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_document_asset_round_trips() {
        let asset = WordDocumentAsset {
            document_name: "My Document.docx".into(),
            text: "Hello, world.".into(),
        };

        let json = serde_json::to_value(&asset).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({
                "document_name": "My Document.docx",
                "text": "Hello, world.",
            }),
        );

        let round_tripped: WordDocumentAsset = serde_json::from_value(json).expect("deserialize");
        assert_eq!(round_tripped, asset);
    }
}
