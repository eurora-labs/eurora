//! `serde_json` round-trip coverage for the YouTube adapter types.
//!
//! Each type's wire form is the contract the server-side LLM-context
//! builder serializes against and the client-side bridge deserializes
//! from. These tests catch accidental `#[serde(rename)]`, field-type,
//! or field-removal drift before it ships.

use eurora_tools_browser::youtube::{CapturedFrame, CurrentTimestamp, Transcript, TranscriptEntry};

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let encoded = serde_json::to_value(value).expect("serialize");
    serde_json::from_value(encoded).expect("deserialize")
}

#[test]
fn current_timestamp_round_trips() {
    let value = CurrentTimestamp {
        video_id: "dQw4w9WgXcQ".into(),
        current_time: 42.5,
        duration: 213.0,
        playing: true,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn transcript_round_trips_with_entries() {
    let value = Transcript {
        video_id: "dQw4w9WgXcQ".into(),
        language: "en-US".into(),
        entries: vec![
            TranscriptEntry {
                start: 0.0,
                duration: 1.5,
                text: "Never gonna give you up".into(),
            },
            TranscriptEntry {
                start: 1.5,
                duration: 1.5,
                text: "Never gonna let you down".into(),
            },
        ],
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn transcript_round_trips_with_no_entries() {
    let value = Transcript {
        video_id: "abc".into(),
        language: "en".into(),
        entries: vec![],
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn captured_frame_round_trips() {
    let value = CapturedFrame {
        video_id: "dQw4w9WgXcQ".into(),
        current_time: 90.25,
        width: 1280,
        height: 720,
        image_base64: "iVBORw0KGgoAAAANSUhEUg==".into(),
    };
    assert_eq!(round_trip(&value), value);
}
