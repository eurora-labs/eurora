//! Tests for HTML utilities.
//!
//! Mirrors `langchain/libs/core/tests/unit_tests/utils/test_html.py`

use agent_chain_core::utils::html::{
    PREFIXES_TO_IGNORE, SUFFIXES_TO_IGNORE, extract_sub_links, find_all_links,
};

#[test]
fn test_find_all_links_none() {
    let html = "<span>Hello world</span>";
    let actual = find_all_links(html, None);
    assert_eq!(actual, Vec::<String>::new());
}

#[test]
fn test_find_all_links_single() {
    let htmls = [
        "href='foobar.com'",
        r#"href="foobar.com""#,
        r#"<div><a class="blah" href="foobar.com">hullo</a></div>"#,
    ];
    for html in htmls {
        let actual = find_all_links(html, None);
        assert_eq!(
            actual,
            vec!["foobar.com".to_string()],
            "Failed for: {}",
            html
        );
    }
}

#[test]
fn test_find_all_links_multiple() {
    let html = r#"<div><a class="blah" href="https://foobar.com">hullo</a></div><div><a class="bleh" href="/baz/cool">buhbye</a></div>"#;
    let mut actual = find_all_links(html, None);
    actual.sort();
    assert_eq!(actual, vec!["/baz/cool", "https://foobar.com"]);
}

#[test]
fn test_find_all_links_ignore_suffix() {
    for suffix in SUFFIXES_TO_IGNORE {
        let html = format!(r#"href="foobar{}""#, suffix);
        let actual = find_all_links(&html, None);
        assert_eq!(
            actual,
            Vec::<String>::new(),
            "Should ignore suffix: {}",
            suffix
        );
    }

    for suffix in SUFFIXES_TO_IGNORE {
        let html = format!(r#"href="foobar{}more""#, suffix);
        let actual = find_all_links(&html, None);
        assert_eq!(
            actual,
            vec![format!("foobar{}more", suffix)],
            "Should NOT ignore suffix {} when not at end",
            suffix
        );
    }
}

#[test]
fn test_find_all_links_ignore_prefix() {
    for prefix in PREFIXES_TO_IGNORE {
        let html = format!(r#"href="{}foobar""#, prefix);
        let actual = find_all_links(&html, None);
        assert_eq!(
            actual,
            Vec::<String>::new(),
            "Should ignore prefix: {}",
            prefix
        );
    }

    for prefix in PREFIXES_TO_IGNORE {
        if *prefix == "#" {
            continue;
        }
        let html = format!(r#"href="foobar{}more""#, prefix);
        let actual = find_all_links(&html, None);
        assert_eq!(
            actual,
            vec![format!("foobar{}more", prefix)],
            "Should NOT ignore prefix {} when not at beginning",
            prefix
        );
    }
}

#[test]
fn test_find_all_links_drop_fragment() {
    let html = r#"href="foobar.com/woah#section_one""#;
    let actual = find_all_links(html, None);
    assert_eq!(actual, vec!["foobar.com/woah".to_string()]);
}

#[test]
fn test_extract_sub_links() {
    let html = r#"<a href="https://foobar.com">one</a><a href="http://baz.net">two</a><a href="//foobar.com/hello">three</a><a href="/how/are/you/doing">four</a>"#;

    let mut expected = vec![
        "https://foobar.com",
        "https://foobar.com/hello",
        "https://foobar.com/how/are/you/doing",
    ];
    expected.sort();

    let mut actual = extract_sub_links(html, "https://foobar.com", None, None, true, &[]);
    actual.sort();
    assert_eq!(actual, expected);

    let actual = extract_sub_links(html, "https://foobar.com/hello", None, None, true, &[]);
    let expected = vec!["https://foobar.com/hello"];
    assert_eq!(actual, expected);

    let mut actual = extract_sub_links(html, "https://foobar.com/hello", None, None, false, &[]);
    actual.sort();
    let mut expected = vec![
        "https://foobar.com",
        "http://baz.net",
        "https://foobar.com/hello",
        "https://foobar.com/how/are/you/doing",
    ];
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn test_extract_sub_links_base() {
    let html = r#"<a href="https://foobar.com">one</a><a href="http://baz.net">two</a><a href="//foobar.com/hello">three</a><a href="/how/are/you/doing">four</a><a href="alexis.html"</a>"#;

    let mut expected = vec![
        "https://foobar.com",
        "https://foobar.com/hello",
        "https://foobar.com/how/are/you/doing",
        "https://foobar.com/hello/alexis.html",
    ];
    expected.sort();

    let mut actual = extract_sub_links(
        html,
        "https://foobar.com/hello/bill.html",
        Some("https://foobar.com"),
        None,
        true,
        &[],
    );
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn test_extract_sub_links_exclude() {
    let html = r#"<a href="https://foobar.com">one</a><a href="http://baz.net">two</a><a href="//foobar.com/hello">three</a><a href="/how/are/you/doing">four</a><a href="alexis.html"</a>"#;

    let mut expected = vec![
        "http://baz.net",
        "https://foobar.com",
        "https://foobar.com/hello",
        "https://foobar.com/hello/alexis.html",
    ];
    expected.sort();

    let mut actual = extract_sub_links(
        html,
        "https://foobar.com/hello/bill.html",
        Some("https://foobar.com"),
        None,
        false,
        &["https://foobar.com/how", "http://baz.org"],
    );
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn test_prevent_outside() {
    let html = r#"<a href="https://foobar.comic.com">BAD</a><a href="https://foobar.comic:9999">BAD</a><a href="https://foobar.com:9999">BAD</a><a href="http://foobar.com:9999/">BAD</a><a href="https://foobar.com/OK">OK</a><a href="http://foobar.com/BAD">BAD</a>"#;

    let mut expected = vec!["https://foobar.com/OK"];
    expected.sort();

    let mut actual = extract_sub_links(
        html,
        "https://foobar.com/hello/bill.html",
        Some("https://foobar.com"),
        None,
        true,
        &[],
    );
    actual.sort();
    assert_eq!(actual, expected);
}

#[test]
fn test_extract_sub_links_with_query() {
    let html = r#"<a href="https://foobar.com?query=123">one</a><a href="/hello?query=456">two</a><a href="//foobar.com/how/are/you?query=789">three</a><a href="doing?query=101112"></a>"#;

    let mut expected = vec![
        "https://foobar.com?query=123",
        "https://foobar.com/hello?query=456",
        "https://foobar.com/how/are/you?query=789",
        "https://foobar.com/hello/doing?query=101112",
    ];
    expected.sort();

    let mut actual = extract_sub_links(
        html,
        "https://foobar.com/hello/bill.html",
        Some("https://foobar.com"),
        None,
        true,
        &[],
    );
    actual.sort();
    assert_eq!(
        actual, expected,
        "Expected {:?}, but got {:?}",
        expected, actual
    );
}
