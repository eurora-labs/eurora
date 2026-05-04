use bon::Builder;
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
/// Construct via [`WordAsset::builder`]; `id` is optional and defaults
/// to a freshly allocated v4 UUID. The [`From<WordDocumentAsset>`] impl
/// is a thin wrapper around the builder for the common "promote a wire
/// payload" path.
///
/// Only the wire shape ([`WordDocumentAsset`]) participates in
/// TypeScript codegen; this type stays Rust-only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[builder(on(String, into))]
pub struct WordAsset {
    #[builder(default = uuid::Uuid::new_v4().to_string())]
    pub id: String,
    pub document_name: String,
    pub text: String,
}

impl From<WordDocumentAsset> for WordAsset {
    fn from(wire: WordDocumentAsset) -> Self {
        Self::builder()
            .document_name(wire.document_name)
            .text(wire.text)
            .build()
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
    fn from_wire_assigns_unique_ids_per_call() {
        let wire = WordDocumentAsset {
            document_name: "Doc.docx".into(),
            text: "body".into(),
        };

        let a = WordAsset::from(wire.clone());
        let b = WordAsset::from(wire);

        assert_ne!(a.id, b.id, "each conversion must allocate a fresh UUID");
        assert_eq!(a.document_name, "Doc.docx");
        assert_eq!(a.text, "body");
    }

    #[test]
    fn builder_auto_mints_uuid_when_id_omitted() {
        let asset = WordAsset::builder()
            .document_name("Notes.docx")
            .text("body")
            .build();
        assert!(
            uuid::Uuid::parse_str(&asset.id).is_ok(),
            "auto-minted id must parse as a UUID, got {:?}",
            asset.id,
        );
    }

    #[test]
    fn word_asset_round_trips() {
        let asset = WordAsset::builder()
            .id(uuid::Uuid::nil().to_string())
            .document_name("Notes.docx")
            .text("Some body text.")
            .build();

        let json = serde_json::to_string(&asset).expect("serialize");
        let round_tripped: WordAsset = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(round_tripped, asset);
    }
}
