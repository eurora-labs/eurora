// use eur_proto::ipc::ProtoTranscriptLine;
use once_cell::sync::Lazy;
use regex::Regex;

// pub fn flatten_transcript_with_highlight(
//     transcript: Vec<ProtoTranscriptLine>,
//     current_time: f32,
//     highlight_tag: String,
// ) -> String {
//     let mut flat_transcript = String::new();
//     let mut highlighted = false;
//     for line in transcript {
//         if line.start > current_time && !highlighted {
//             flat_transcript += &highlight_tag;
//             flat_transcript += &line.text;
//             flat_transcript += &highlight_tag;
//             highlighted = true;
//         } else {
//             flat_transcript += &line.text.to_string();
//         }
//     }
//     flat_transcript
// }

static EMAIL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b").unwrap());
pub fn redact_emails<S: AsRef<str>>(input: S) -> String {
    EMAIL_RE
        .replace_all(input.as_ref(), "<REDACTED>")
        .into_owned()
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let original = "Contact me: user.name+tag@example.co.uk and admin@domain.com.";
        let expected = "Contact me: <REDACTED> and <REDACTED>.";
        assert_eq!(redact_emails(original), expected);
    }
}
