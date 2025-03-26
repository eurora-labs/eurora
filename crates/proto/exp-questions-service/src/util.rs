//! Utility functions for transcript processing

/// Flattens a transcript into a single string.
///
/// # Arguments
///
/// * `transcript` - The transcript as a Vec of dictionaries with "start", "text" keys
/// * `last_timestamp` - Optional timestamp to stop including lines
/// * `include_timestamps` - Whether to include timestamps in the output
#[allow(dead_code)]
pub fn flatten_transcript(
    transcript: &[serde_json::Value],
    last_timestamp: Option<f32>,
    include_timestamps: bool,
) -> String {
    let mut flat_transcript = String::new();

    for line in transcript {
        let start = line["start"].as_f64().unwrap_or(0.0) as f32;

        if let Some(last_ts) = last_timestamp {
            if start > last_ts {
                break;
            }
        }

        if include_timestamps {
            flat_transcript.push_str(&format!(
                "{} - {} ",
                start,
                line["text"].as_str().unwrap_or("")
            ));
        } else {
            flat_transcript.push_str(&format!("{} ", line["text"].as_str().unwrap_or("")));
        }
    }

    // Replace newlines with spaces
    flat_transcript.replace('\n', " ")
}

/// Flattens a transcript into a single string with highlighting for the current line.
///
/// # Arguments
///
/// * `transcript` - The transcript as a Vec of dictionaries with "start", "text" keys
/// * `last_timestamp` - Timestamp to determine which line to highlight
/// * `highlight_tag` - Tag to use for highlighting (default: "%HIGHLIGHT%")
pub fn flatten_transcript_with_highlight(
    transcript: &[serde_json::Value],
    last_timestamp: f32,
    highlight_tag: Option<&str>,
) -> String {
    let highlight_tag = highlight_tag.unwrap_or("%HIGHLIGHT%");
    let mut flat_transcript = String::new();
    let mut highlighted = false;

    for line in transcript {
        let start = line["start"].as_f64().unwrap_or(0.0) as f32;
        let text = line["text"].as_str().unwrap_or("");

        if start > last_timestamp && !highlighted {
            flat_transcript.push_str(&format!("{}{}{} ", highlight_tag, text, highlight_tag));
            highlighted = true;
            continue;
        }

        flat_transcript.push_str(&format!("{} ", text));
    }

    // Replace newlines with spaces
    flat_transcript.replace('\n', " ")
}
