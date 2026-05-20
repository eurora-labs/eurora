//! `serde_json` round-trip coverage for the web adapter types.
//!
//! Each type's wire form is the contract the server-side LLM-context
//! builder serialises against and the client-side bridge deserialises
//! from. These tests catch accidental `#[serde(rename)]`, field-type,
//! default-handling, or field-removal drift before it ships.

use std::collections::HashMap;

use eurora_tools_web::{
    AccessibilityTree, AxNode, BoundingBox, DomNode, FormInput, FormInputKind, FormInputsList,
    GetAccessibilityTreeArgs, InsertTextArgs, InsertTextResult, Link, LinksList,
    ListFormInputsArgs, ListLinksArgs, PageMetadata, QuerySelectorArgs, QuerySelectorInclude,
    QuerySelectorResult, ReadabilityArticle, SelectedText, ViewportMetrics,
};
use serde_json::json;

fn round_trip<T>(value: &T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let encoded = serde_json::to_value(value).expect("serialize");
    serde_json::from_value(encoded).expect("deserialize")
}

#[test]
fn page_metadata_round_trips_with_optional_fields_populated() {
    let mut og = HashMap::new();
    og.insert("title".into(), "Example domain".into());
    og.insert("image".into(), "https://example.com/banner.png".into());

    let value = PageMetadata {
        url: "https://example.com/path?q=1#frag".into(),
        title: "Example domain".into(),
        host: "example.com".into(),
        language: Some("en-US".into()),
        charset: Some("utf-8".into()),
        description: Some("Illustrative example".into()),
        og,
        viewport: ViewportMetrics {
            scroll_x: 0.0,
            scroll_y: 120.5,
            inner_width: 1280.0,
            inner_height: 800.0,
            document_height: 4200.0,
        },
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn page_metadata_round_trips_with_optional_fields_absent() {
    let value = PageMetadata {
        url: "https://example.com".into(),
        title: String::new(),
        host: "example.com".into(),
        language: None,
        charset: None,
        description: None,
        og: HashMap::new(),
        viewport: ViewportMetrics {
            scroll_x: 0.0,
            scroll_y: 0.0,
            inner_width: 800.0,
            inner_height: 600.0,
            document_height: 600.0,
        },
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn get_accessibility_tree_args_round_trip_with_defaults() {
    let value = GetAccessibilityTreeArgs::default();
    assert_eq!(round_trip(&value), value);
}

#[test]
fn get_accessibility_tree_args_round_trip_with_all_fields() {
    let value = GetAccessibilityTreeArgs {
        root_selector: Some("main".into()),
        max_depth: Some(8),
        max_nodes: Some(1_000),
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn accessibility_tree_round_trips_with_nested_children() {
    let leaf = AxNode {
        role: "button".into(),
        name: Some("Submit".into()),
        value: None,
        description: Some("Submits the form".into()),
        selector_path: Some("form > button[type='submit']".into()),
        children: vec![],
    };
    let value = AccessibilityTree {
        root: AxNode {
            role: "main".into(),
            name: None,
            value: None,
            description: None,
            selector_path: Some("main".into()),
            children: vec![leaf],
        },
        node_count: 2,
        truncated: false,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn readability_article_round_trips() {
    let value = ReadabilityArticle {
        title: Some("Tokio".into()),
        byline: None,
        site_name: Some("Wikipedia".into()),
        language: Some("en".into()),
        excerpt: Some("Tokio is a runtime for Rust.".into()),
        content_html: "<p>Tokio is a runtime for Rust.</p>".into(),
        text_content: "Tokio is a runtime for Rust.".into(),
        length: 29,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn selected_text_round_trips() {
    let value = SelectedText {
        text: "hello".into(),
        anchor_xpath: Some("/html/body/p[1]/text()[1]".into()),
        focus_xpath: Some("/html/body/p[1]/text()[1]".into()),
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn query_selector_args_round_trip_with_explicit_limit_and_include() {
    let value = QuerySelectorArgs {
        selector: "a.cta".into(),
        limit: 10,
        include: vec![
            QuerySelectorInclude::Text,
            QuerySelectorInclude::Bounds,
            QuerySelectorInclude::Attributes,
        ],
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn query_selector_args_apply_defaults_when_omitted() {
    let decoded: QuerySelectorArgs = serde_json::from_value(json!({ "selector": "a" }))
        .expect("decode with missing limit/include");
    assert_eq!(decoded.selector, "a");
    assert_eq!(decoded.limit, 50);
    assert!(decoded.include.is_empty());
}

#[test]
fn query_selector_include_serialises_as_snake_case() {
    let value = vec![
        QuerySelectorInclude::Text,
        QuerySelectorInclude::Html,
        QuerySelectorInclude::Attributes,
        QuerySelectorInclude::Bounds,
    ];
    assert_eq!(
        serde_json::to_value(&value).unwrap(),
        json!(["text", "html", "attributes", "bounds"])
    );
}

#[test]
fn dom_node_round_trips_with_all_facets_populated() {
    let mut attributes = HashMap::new();
    attributes.insert("href".into(), "/about".into());
    attributes.insert("class".into(), "cta".into());
    let value = DomNode {
        selector_path: "a.cta:nth-of-type(1)".into(),
        text: Some("About".into()),
        html: Some("<a class=\"cta\" href=\"/about\">About</a>".into()),
        attributes: Some(attributes),
        bounds: Some(BoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 32.0,
        }),
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn dom_node_round_trips_with_only_selector_path() {
    let value = DomNode {
        selector_path: "div#main".into(),
        text: None,
        html: None,
        attributes: None,
        bounds: None,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn query_selector_result_round_trips() {
    let value = QuerySelectorResult {
        matches: vec![DomNode {
            selector_path: "h1".into(),
            text: Some("Title".into()),
            html: None,
            attributes: None,
            bounds: None,
        }],
        total_match_count: 3,
        truncated: true,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn links_list_round_trips() {
    let value = LinksList {
        links: vec![Link {
            url: "https://example.com/about".into(),
            label: Some("About".into()),
            role: "link".into(),
            selector_path: "header a:nth-of-type(2)".into(),
        }],
        total: 1,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn list_links_args_apply_defaults_when_omitted() {
    let decoded: ListLinksArgs = serde_json::from_value(json!({})).expect("decode empty object");
    assert_eq!(decoded.root_selector, None);
    assert_eq!(decoded.limit, 100);
}

#[test]
fn form_inputs_list_round_trips_across_every_kind() {
    let kinds = [
        FormInputKind::Text,
        FormInputKind::Search,
        FormInputKind::Email,
        FormInputKind::Url,
        FormInputKind::Tel,
        FormInputKind::Number,
        FormInputKind::Textarea,
        FormInputKind::ContentEditable,
    ];
    let inputs: Vec<_> = kinds
        .into_iter()
        .enumerate()
        .map(|(i, kind)| FormInput {
            field_id: format!("#field-{i}"),
            label: Some(format!("Field {i}")),
            kind,
            value: String::new(),
            placeholder: None,
            required: false,
        })
        .collect();
    let value = FormInputsList {
        inputs,
        total: kinds.len() as u32,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn form_input_kind_serialises_as_snake_case() {
    assert_eq!(
        serde_json::to_value(FormInputKind::ContentEditable).unwrap(),
        json!("content_editable")
    );
    assert_eq!(
        serde_json::to_value(FormInputKind::Textarea).unwrap(),
        json!("textarea")
    );
}

#[test]
fn list_form_inputs_args_apply_defaults_when_omitted() {
    let decoded: ListFormInputsArgs =
        serde_json::from_value(json!({ "root_selector": "form#login" }))
            .expect("decode partial args");
    assert_eq!(decoded.root_selector.as_deref(), Some("form#login"));
    assert_eq!(decoded.limit, 100);
}

#[test]
fn insert_text_args_default_replace_to_false() {
    let decoded: InsertTextArgs = serde_json::from_value(json!({ "field_id": "#q", "text": "hi" }))
        .expect("decode without replace");
    assert!(!decoded.replace);
    assert_eq!(decoded.field_id, "#q");
    assert_eq!(decoded.text, "hi");
}

#[test]
fn insert_text_args_round_trip_with_replace_set() {
    let value = InsertTextArgs {
        field_id: "#q".into(),
        text: "hello".into(),
        replace: true,
    };
    assert_eq!(round_trip(&value), value);
}

#[test]
fn insert_text_result_round_trips() {
    let value = InsertTextResult {
        field_id: "#q".into(),
        previous_value: "old".into(),
        new_value: "old hello".into(),
    };
    assert_eq!(round_trip(&value), value);
}
