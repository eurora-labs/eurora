use eur_proto::ipc::ProtoTranscriptLine;

pub fn flatten_transcript_with_highlight(
    transcript: Vec<ProtoTranscriptLine>,
    current_time: f32,
    highlight_tag: String,
) -> String {
    let mut flat_transcript = String::new();
    let mut highlighted = false;
    for line in transcript {
        if line.start > current_time && !highlighted {
            flat_transcript += &highlight_tag;
            flat_transcript += &line.text;
            flat_transcript += &highlight_tag;
            highlighted = true;
        } else {
            flat_transcript += &line.text.to_string();
        }
    }
    flat_transcript
}
