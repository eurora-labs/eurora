use regex::Regex;
use std::collections::HashSet;

pub const PREFIXES_TO_IGNORE: &[&str] = &["javascript:", "mailto:", "#"];

pub const SUFFIXES_TO_IGNORE: &[&str] = &[
    ".css", ".js", ".ico", ".png", ".jpg", ".jpeg", ".gif", ".svg", ".csv", ".bz2", ".zip", ".epub",
];

pub fn default_link_regex() -> Regex {
    Regex::new(r#"href=["']([^"'#]+)["'#]"#).expect("Failed to compile default link regex")
}

fn should_ignore_prefix(link: &str) -> bool {
    PREFIXES_TO_IGNORE
        .iter()
        .any(|prefix| link.starts_with(prefix))
}

fn should_ignore_suffix(link: &str) -> bool {
    SUFFIXES_TO_IGNORE
        .iter()
        .any(|suffix| link.ends_with(suffix))
}

pub fn find_all_links(raw_html: &str, pattern: Option<&Regex>) -> Vec<String> {
    let default_regex = default_link_regex();
    let regex = pattern.unwrap_or(&default_regex);

    regex
        .captures_iter(raw_html)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        .filter(|link| !should_ignore_prefix(link) && !should_ignore_suffix(link))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect()
}

pub fn extract_sub_links(
    raw_html: &str,
    url: &str,
    base_url: Option<&str>,
    pattern: Option<&Regex>,
    prevent_outside: bool,
    exclude_prefixes: &[&str],
) -> Vec<String> {
    let base_url_to_use = base_url.unwrap_or(url);

    let parsed_base_url = match url::Url::parse(base_url_to_use) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    let parsed_url = match url::Url::parse(url) {
        Ok(u) => u,
        Err(_) => return vec![],
    };

    let all_links = find_all_links(raw_html, pattern);
    let mut absolute_paths = HashSet::new();

    for link in all_links {
        let absolute_path = match url::Url::parse(&link) {
            Ok(parsed_link) => {
                if parsed_link.scheme() == "http" || parsed_link.scheme() == "https" {
                    link
                } else {
                    continue;
                }
            }
            Err(_) => {
                if link.starts_with("//") {
                    format!("{}:{}", parsed_url.scheme(), link)
                } else {
                    match parsed_url.join(&link) {
                        Ok(joined) => joined.to_string(),
                        Err(_) => continue,
                    }
                }
            }
        };

        absolute_paths.insert(absolute_path);
    }

    let mut results = Vec::new();

    for path in absolute_paths {
        if exclude_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            continue;
        }

        if prevent_outside {
            let parsed_path = match url::Url::parse(&path) {
                Ok(u) => u,
                Err(_) => continue,
            };

            let base_netloc = format!(
                "{}{}",
                parsed_base_url.host_str().unwrap_or(""),
                parsed_base_url
                    .port()
                    .map_or(String::new(), |p| format!(":{}", p))
            );
            let path_netloc = format!(
                "{}{}",
                parsed_path.host_str().unwrap_or(""),
                parsed_path
                    .port()
                    .map_or(String::new(), |p| format!(":{}", p))
            );

            if base_netloc != path_netloc {
                continue;
            }

            if !path.starts_with(base_url_to_use) {
                continue;
            }
        }

        results.push(path);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_all_links() {
        let html = r#"
            <a href="https://example.com/page1">Link 1</a>
            <a href="/page2">Link 2</a>
            <a href="https://example.com/page1">Duplicate</a>
        "#;

        let links = find_all_links(html, None);
        assert!(links.contains(&"https://example.com/page1".to_string()));
        assert!(links.contains(&"/page2".to_string()));
    }

    #[test]
    fn test_find_all_links_ignores_prefixes() {
        let html = r##"
            <a href="javascript:void(0)">JS Link</a>
            <a href="mailto:test@example.com">Email</a>
            <a href="#section">Anchor</a>
            <a href="https://example.com">Valid</a>
        "##;

        let links = find_all_links(html, None);
        assert!(!links.iter().any(|l| l.starts_with("javascript:")));
        assert!(!links.iter().any(|l| l.starts_with("mailto:")));
        assert!(!links.iter().any(|l| l.starts_with("#")));
    }

    #[test]
    fn test_find_all_links_ignores_suffixes() {
        let html = r##"
            <a href="style.css">CSS</a>
            <a href="script.js">JS</a>
            <a href="image.png">Image</a>
            <a href="https://example.com/page">Valid</a>
        "##;

        let links = find_all_links(html, None);
        assert!(!links.iter().any(|l| l.ends_with(".css")));
        assert!(!links.iter().any(|l| l.ends_with(".js")));
        assert!(!links.iter().any(|l| l.ends_with(".png")));
    }

    #[test]
    fn test_extract_sub_links() {
        let html = r#"
            <a href="/page1">Link 1</a>
            <a href="https://example.com/page2">Link 2</a>
        "#;

        let links = extract_sub_links(html, "https://example.com", None, None, true, &[]);

        for link in &links {
            assert!(link.starts_with("https://example.com"));
        }
    }

    #[test]
    fn test_extract_sub_links_prevent_outside() {
        let html = r#"
            <a href="https://example.com/page">Internal</a>
            <a href="https://other.com/page">External</a>
        "#;

        let links = extract_sub_links(html, "https://example.com", None, None, true, &[]);

        assert!(links.iter().any(|l| l.contains("example.com")));
        assert!(!links.iter().any(|l| l.contains("other.com")));
    }

    #[test]
    fn test_extract_sub_links_exclude_prefixes() {
        let html = r#"
            <a href="https://example.com/api/v1">API</a>
            <a href="https://example.com/page">Page</a>
        "#;

        let links = extract_sub_links(
            html,
            "https://example.com",
            None,
            None,
            false,
            &["https://example.com/api"],
        );

        assert!(!links.iter().any(|l| l.contains("/api/")));
        assert!(links.iter().any(|l| l.contains("/page")));
    }
}
