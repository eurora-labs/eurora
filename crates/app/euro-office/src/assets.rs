use serde::{Deserialize, Serialize};
use specta::Type;

/// Snapshot of a Word document's textual content as it travels over the
/// bridge.
///
/// Sent by the Office add-in's runtime as the JSON payload of a
/// `ResponseFrame` for `GET_ASSETS`. The desktop deserializes this
/// directly — there is no `NativeMessage` discriminator wrapper because
/// the WebSocket transport (unlike Chrome's stdio native-messaging)
/// does not require a single envelope shape per message.
///
/// This is a pure wire type: it derives [`specta::Type`] so it appears in
/// the TypeScript bindings consumed by the add-in. The desktop wraps it
/// in a [`WordAsset`] (with a stable UUID) before storing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct WordDocumentAsset {
    /// Document title as reported by `Word.Document.properties.title`,
    /// or a fallback string when the document is unsaved/untitled.
    pub document_name: String,
    /// Full document body text (`Word.Document.body.text`).
    pub text: String,
}

/// Desktop-side representation of a Word document asset.
///
/// Wraps a [`WordDocumentAsset`] and assigns a stable identifier so the
/// rest of the activity pipeline can dedupe, store, and reference the
/// asset independently of the document's title or content.
///
/// Only the wire shape ([`WordDocumentAsset`]) participates in
/// TypeScript codegen; this type stays Rust-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WordAsset {
    pub id: String,
    pub document_name: String,
    pub text: String,
}

impl WordAsset {
    /// Build a [`WordAsset`] from already-known fields, e.g. for tests
    /// or when reconstructing from storage.
    pub fn new(id: String, document_name: String, text: String) -> Self {
        Self {
            id,
            document_name,
            text,
        }
    }
}

impl From<WordDocumentAsset> for WordAsset {
    fn from(wire: WordDocumentAsset) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            document_name: wire.document_name,
            text: wire.text,
        }
    }
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

    #[test]
    fn word_asset_assigns_unique_ids_when_built_from_wire() {
        let wire = WordDocumentAsset {
            document_name: "Doc.docx".into(),
            text: "body".into(),
        };

        let a = WordAsset::from(wire.clone());
        let b = WordAsset::from(wire);

        assert_ne!(a.id, b.id, "each wrap must allocate a fresh UUID");
        assert_eq!(a.document_name, "Doc.docx");
        assert_eq!(a.text, "body");
    }

    #[test]
    fn word_asset_round_trips() {
        let asset = WordAsset::new(
            "11111111-1111-1111-1111-111111111111".into(),
            "Notes.docx".into(),
            "Some body text.".into(),
        );

        let json = serde_json::to_string(&asset).expect("serialize");
        let round_tripped: WordAsset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_tripped, asset);
    }
}
