use std::collections::BTreeMap;

use agent_chain::AnyMessage;
use agent_chain::messages::{ContentBlock, ImageContentBlock, TextContentBlock};

/// Scan every `HumanMessage` in `messages` and return the image blocks keyed by the
/// identifier that the main LLM will see in the placeholder text. The ordered map keeps
/// the placement deterministic so the system prompt lists ids in a stable order.
pub(crate) fn collect_thread_images(
    messages: &[AnyMessage],
) -> BTreeMap<String, ImageContentBlock> {
    let mut out = BTreeMap::new();
    for message in messages {
        let AnyMessage::HumanMessage(human) = message else {
            continue;
        };
        for block in human.content.iter() {
            if let ContentBlock::Image(image) = block
                && let Some(id) = image_identifier(image)
            {
                out.entry(id).or_insert_with(|| image.clone());
            }
        }
    }
    out
}

pub(crate) fn image_identifier(image: &ImageContentBlock) -> Option<String> {
    image.file_id.clone().or_else(|| image.id.clone())
}

/// Replace every `ContentBlock::Image` inside a `HumanMessage` with a text placeholder so
/// the text-only main model never receives image bytes. The placeholder exposes the
/// `image_id` that the model must pass to the `describe_image` tool.
pub(crate) fn project_for_text_llm(messages: &mut [AnyMessage]) {
    for message in messages.iter_mut() {
        let AnyMessage::HumanMessage(human) = message else {
            continue;
        };
        for block in human.content.iter_mut() {
            let placeholder = match block {
                ContentBlock::Image(image) => render_image_placeholder(image),
                _ => continue,
            };
            *block = ContentBlock::Text(TextContentBlock::builder().text(placeholder).build());
        }
    }
}

fn render_image_placeholder(image: &ImageContentBlock) -> String {
    let id = image_identifier(image).unwrap_or_else(|| "<unknown>".to_string());
    let mime = image.mime_type.as_deref().unwrap_or("unknown");
    format!(
        "[attached image — image_id: {id}, mime_type: {mime}. You cannot see this image. \
         Call the `describe_image` tool with this image_id and a specific question to learn \
         about it.]"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_chain::HumanMessage;

    fn image_block(file_id: &str, mime: &str) -> ImageContentBlock {
        ImageContentBlock::builder()
            .file_id(file_id.to_string())
            .url(format!("s3://bucket/{file_id}"))
            .mime_type(mime.to_string())
            .build()
            .expect("image block builder should succeed with file_id")
    }

    fn text_block(text: &str) -> ContentBlock {
        ContentBlock::Text(TextContentBlock::builder().text(text).build())
    }

    #[test]
    fn collect_thread_images_finds_images_across_human_messages() {
        let messages = vec![
            HumanMessage::builder()
                .content(vec![
                    text_block("here is one"),
                    ContentBlock::Image(image_block("img-a", "image/png")),
                ])
                .build()
                .into(),
            HumanMessage::builder()
                .content(vec![
                    ContentBlock::Image(image_block("img-b", "image/jpeg")),
                    text_block("and another"),
                ])
                .build()
                .into(),
        ];

        let images = collect_thread_images(&messages);

        assert_eq!(images.len(), 2);
        assert!(images.contains_key("img-a"));
        assert!(images.contains_key("img-b"));
    }

    #[test]
    fn collect_thread_images_deduplicates_by_identifier() {
        let duplicated = image_block("dup", "image/png");
        let messages = vec![
            HumanMessage::builder()
                .content(vec![ContentBlock::Image(duplicated.clone())])
                .build()
                .into(),
            HumanMessage::builder()
                .content(vec![ContentBlock::Image(duplicated)])
                .build()
                .into(),
        ];

        let images = collect_thread_images(&messages);

        assert_eq!(images.len(), 1);
    }

    #[test]
    fn collect_thread_images_skips_images_without_identifier() {
        let anonymous = ImageContentBlock::builder()
            .url("s3://bucket/anonymous".to_string())
            .mime_type("image/png".to_string())
            .build()
            .unwrap();
        let messages = vec![
            HumanMessage::builder()
                .content(vec![ContentBlock::Image(anonymous)])
                .build()
                .into(),
        ];

        assert!(collect_thread_images(&messages).is_empty());
    }

    #[test]
    fn project_for_text_llm_replaces_image_blocks_with_placeholders() {
        let mut messages = vec![
            HumanMessage::builder()
                .content(vec![
                    text_block("look at this"),
                    ContentBlock::Image(image_block("img-a", "image/png")),
                ])
                .build()
                .into(),
        ];

        project_for_text_llm(&mut messages);

        let AnyMessage::HumanMessage(human) = &messages[0] else {
            panic!("expected human message");
        };
        let blocks: Vec<&ContentBlock> = human.content.iter().collect();
        assert!(matches!(blocks[0], ContentBlock::Text(_)));
        let ContentBlock::Text(placeholder) = blocks[1] else {
            panic!("expected text placeholder");
        };
        assert!(placeholder.text.contains("image_id: img-a"));
        assert!(placeholder.text.contains("image/png"));
        assert!(placeholder.text.contains("describe_image"));
    }

    #[test]
    fn image_identifier_prefers_file_id_over_id() {
        let image = ImageContentBlock::builder()
            .id("block-id".to_string())
            .file_id("asset-file-id".to_string())
            .url("s3://bucket/x".to_string())
            .build()
            .unwrap();
        assert_eq!(image_identifier(&image).as_deref(), Some("asset-file-id"));
    }
}
