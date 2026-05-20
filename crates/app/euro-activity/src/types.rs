use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use thread_core::ContextChip;
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: Uuid,
    pub name: String,
    pub title: Option<String>,
    pub url: Option<Url>,
    #[serde(skip)]
    pub icon: Option<Arc<image::RgbaImage>>,
    pub process_name: String,
    pub process_id: u32,
    pub start: DateTime<Utc>,
    pub end: Option<DateTime<Utc>>,
}

impl Activity {
    pub fn new(
        name: String,
        title: Option<String>,
        icon: Option<Arc<image::RgbaImage>>,
        process_name: String,
        process_id: u32,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            name,
            title,
            url: None,
            icon,
            process_name,
            process_id,
            start: Utc::now(),
            end: None,
        }
    }

    /// Construct an Activity that represents a focused browser page.
    ///
    /// A browser Activity is one whose `url` is always set to a parsed URL,
    /// which in turn guarantees that `get_context_chip` emits a meaningful
    /// `domain`. Callers that cannot parse a URL must fall back to
    /// [`Activity::new`] instead of passing synthetic or empty values here.
    pub fn new_browser(
        url: Url,
        title: Option<String>,
        icon: Option<Arc<image::RgbaImage>>,
        process_name: String,
        process_id: u32,
    ) -> Self {
        Self {
            id: Uuid::now_v7(),
            name: url.to_string(),
            title,
            url: Some(url),
            icon,
            process_name,
            process_id,
            start: Utc::now(),
            end: None,
        }
    }

    pub fn get_context_chip(&self) -> ContextChip {
        ContextChip {
            id: self.id.to_string(),
            name: self.window_title(),
            icon: None,
            domain: self.url.as_ref().and_then(domain_from_url),
        }
    }

    /// Title to render alongside the activity. `title` is the OS-reported
    /// window title; when absent (some platforms / strategies don't
    /// populate it) we fall back to `name`, which is always set.
    pub fn window_title(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.name.clone())
    }

    /// Replace the URL and the URL-derived `name` in one step.
    ///
    /// Keeps `name` and `url` in sync so that downstream consumers (e.g.
    /// storage, which uses `name`) and `get_context_chip` (which uses `url`)
    /// never disagree about which page the Activity refers to.
    pub fn set_url(&mut self, url: Url) {
        self.name = url.to_string();
        self.url = Some(url);
    }

    pub fn end_activity(&mut self) {
        self.end = Some(Utc::now());
    }
}

fn domain_from_url(url: &Url) -> Option<String> {
    let host = url.host_str()?.to_ascii_lowercase();
    Some(host.strip_prefix("www.").unwrap_or(&host).to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> Url {
        Url::parse(input).expect("valid test URL")
    }

    #[test]
    fn extracts_bare_host() {
        assert_eq!(
            domain_from_url(&parse("https://x.com/some/path")),
            Some("x.com".into())
        );
    }

    #[test]
    fn strips_www_and_lowercases() {
        assert_eq!(
            domain_from_url(&parse("https://WWW.Example.COM/")),
            Some("example.com".into())
        );
    }

    #[test]
    fn preserves_subdomains() {
        assert_eq!(
            domain_from_url(&parse("https://m.youtube.com/watch?v=1")),
            Some("m.youtube.com".into())
        );
    }

    #[test]
    fn browser_activity_has_domain_in_chip() {
        let activity = Activity::new_browser(
            parse("https://youtube.com/watch?v=abc"),
            Some("Great Video".into()),
            None,
            "chrome".into(),
            42,
        );
        let chip = activity.get_context_chip();
        assert_eq!(chip.domain.as_deref(), Some("youtube.com"));
        assert_eq!(chip.name, "Great Video");
        assert_eq!(activity.process_id, 42);
    }

    #[test]
    fn non_browser_activity_has_no_domain() {
        let activity = Activity::new("Some Window Title".into(), None, None, "someapp".into(), 7);
        let chip = activity.get_context_chip();
        assert_eq!(chip.domain, None);
        assert_eq!(activity.process_id, 7);
    }

    #[test]
    fn set_url_keeps_name_in_sync() {
        let mut activity = Activity::new_browser(
            parse("https://example.com/a"),
            None,
            None,
            "chrome".into(),
            0,
        );
        let new_url = parse("https://example.com/b");
        activity.set_url(new_url.clone());
        assert_eq!(activity.url.as_ref(), Some(&new_url));
        assert_eq!(activity.name, new_url.to_string());
    }
}
