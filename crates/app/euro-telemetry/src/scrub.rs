//! `before_send` hook that strips the user's home-directory prefix
//! from every string-bearing field of an outgoing Sentry event.
//!
//! Sentry already redacts most server-side data with
//! `send_default_pii=false`, but local file paths are emitted verbatim
//! from `tracing`, the panic hook, and `debug-images`. This walks:
//! top-level `message`, every breadcrumb's `message` and `data` map,
//! every exception's `value` and both stacktraces (frames' `filename`
//! / `abs_path` / `context_line` / `pre_context` / `post_context` /
//! `vars`), the top-level `stacktrace`, and `extra`. Anything
//! `sentry-tracing` puts into `breadcrumb.data` (e.g.
//! `tracing::error!(path = ?some_pathbuf)`) is therefore covered.

use sentry::protocol::{Event, Map, Stacktrace};

/// `before_send` callback registered against `sentry::ClientOptions`.
/// Looks up the home directory once, then walks the event in place.
pub(crate) fn scrub_event(mut event: Event<'static>) -> Option<Event<'static>> {
    let Some(home) = dirs::home_dir() else {
        return Some(event);
    };
    let home = home.to_string_lossy().into_owned();
    if home.is_empty() {
        return Some(event);
    }
    scrub_event_with_home(&mut event, &home);
    Some(event)
}

/// Pure scrubber over an arbitrary "home" needle. Production calls
/// this through `scrub_event` (which sources `home` from
/// `dirs::home_dir`); tests call it directly with a synthetic
/// `/home/test` so they don't depend on the runner's `$HOME`.
fn scrub_event_with_home(event: &mut Event<'static>, home: &str) {
    if let Some(message) = event.message.as_mut() {
        scrub_string(message, home);
    }
    for breadcrumb in &mut event.breadcrumbs {
        if let Some(message) = breadcrumb.message.as_mut() {
            scrub_string(message, home);
        }
        scrub_value_map(&mut breadcrumb.data, home);
    }
    for exception in &mut event.exception {
        if let Some(value) = exception.value.as_mut() {
            scrub_string(value, home);
        }
        if let Some(stacktrace) = exception.stacktrace.as_mut() {
            scrub_stacktrace(stacktrace, home);
        }
        if let Some(stacktrace) = exception.raw_stacktrace.as_mut() {
            scrub_stacktrace(stacktrace, home);
        }
    }
    if let Some(stacktrace) = event.stacktrace.as_mut() {
        scrub_stacktrace(stacktrace, home);
    }
    scrub_value_map(&mut event.extra, home);
}

fn scrub_string(s: &mut String, home: &str) {
    // `String::replace("", _)` inserts the replacement between every
    // character — guard against the degenerate case so a malformed
    // call site can't corrupt event data.
    if home.is_empty() || !s.contains(home) {
        return;
    }
    *s = s.replace(home, "~");
}

fn scrub_stacktrace(stacktrace: &mut Stacktrace, home: &str) {
    for frame in &mut stacktrace.frames {
        if let Some(s) = frame.filename.as_mut() {
            scrub_string(s, home);
        }
        if let Some(s) = frame.abs_path.as_mut() {
            scrub_string(s, home);
        }
        if let Some(s) = frame.context_line.as_mut() {
            scrub_string(s, home);
        }
        for line in &mut frame.pre_context {
            scrub_string(line, home);
        }
        for line in &mut frame.post_context {
            scrub_string(line, home);
        }
        scrub_value_map(&mut frame.vars, home);
    }
}

fn scrub_value_map(map: &mut Map<String, serde_json::Value>, home: &str) {
    for value in map.values_mut() {
        scrub_value(value, home);
    }
}

fn scrub_value(value: &mut serde_json::Value, home: &str) {
    match value {
        serde_json::Value::String(s) => scrub_string(s, home),
        serde_json::Value::Array(items) => items.iter_mut().for_each(|v| scrub_value(v, home)),
        serde_json::Value::Object(map) => map.values_mut().for_each(|v| scrub_value(v, home)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::{Breadcrumb, Exception, Frame};
    use serde_json::json;

    /// Build a fixture event whose every string-bearing field carries
    /// `/home/test` somewhere, so a single scrub call can prove
    /// coverage of all the paths we care about.
    fn fixture_event() -> Event<'static> {
        let frame = Frame {
            filename: Some("/home/test/src/lib.rs".to_owned()),
            abs_path: Some("/home/test/src/lib.rs".to_owned()),
            context_line: Some("    let path = \"/home/test/db\";".to_owned()),
            pre_context: vec!["// /home/test/comment".to_owned()],
            post_context: vec!["// trailing /home/test".to_owned()],
            vars: [("path".to_owned(), json!("/home/test/db"))]
                .into_iter()
                .collect(),
            ..Default::default()
        };
        let stacktrace = Stacktrace {
            frames: vec![frame],
            ..Default::default()
        };
        let exception = Exception {
            ty: "Panic".to_owned(),
            value: Some("crashed reading /home/test/secrets".to_owned()),
            stacktrace: Some(stacktrace.clone()),
            raw_stacktrace: Some(stacktrace),
            ..Default::default()
        };
        let breadcrumb = Breadcrumb {
            message: Some("opened /home/test/file".to_owned()),
            data: [
                ("path".to_owned(), json!("/home/test/db")),
                (
                    "nested".to_owned(),
                    json!({"deeper": ["/home/test/inside", 42]}),
                ),
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };

        Event {
            message: Some("top-level /home/test/here".to_owned()),
            breadcrumbs: vec![breadcrumb].into(),
            exception: vec![exception].into(),
            extra: [("path".to_owned(), json!("/home/test/extra"))]
                .into_iter()
                .collect(),
            ..Default::default()
        }
    }

    fn assert_no_home(event: &Event<'static>, home: &str) {
        let serialized = serde_json::to_string(event).unwrap();
        assert!(
            !serialized.contains(home),
            "home path leaked through serialized event: {serialized}"
        );
    }

    #[test]
    fn scrubs_every_string_field() {
        let mut event = fixture_event();
        scrub_event_with_home(&mut event, "/home/test");
        assert_no_home(&event, "/home/test");
        // Spot-check the replacement marker is present where we expect.
        assert!(
            event.message.as_deref().unwrap().contains("~/here"),
            "message should be rewritten with ~",
        );
    }

    #[test]
    fn leaves_unrelated_strings_alone() {
        let mut event = Event::<'static> {
            message: Some("nothing to redact here".to_owned()),
            ..Default::default()
        };
        let before = event.message.clone();
        scrub_event_with_home(&mut event, "/home/test");
        assert_eq!(event.message, before);
    }

    #[test]
    fn handles_deeply_nested_breadcrumb_data() {
        let mut event = fixture_event();
        scrub_event_with_home(&mut event, "/home/test");
        let breadcrumb = event.breadcrumbs.first().unwrap();
        let nested = &breadcrumb.data["nested"]["deeper"][0];
        assert_eq!(nested.as_str(), Some("~/inside"));
    }

    #[test]
    fn empty_home_is_a_no_op() {
        let mut s = "/home/test/foo".to_owned();
        scrub_string(&mut s, "");
        assert_eq!(s, "/home/test/foo");
    }
}
