use agent_chain_core::documents::Document;
use agent_chain_core::text_splitters::html::{HTMLHeaderTextSplitter, HTMLSectionSplitter};
use std::collections::HashMap;

fn doc(content: &str, metadata: Vec<(&str, &str)>) -> Document {
    let meta: HashMap<String, serde_json::Value> = metadata
        .into_iter()
        .map(|(k, v)| (k.to_string(), serde_json::Value::String(v.to_string())))
        .collect();
    Document::builder()
        .page_content(content.to_string())
        .metadata(meta)
        .build()
}

fn assert_docs_eq(actual: &[Document], expected: &[Document], test_case: &str) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "Test Case '{}' Failed: Number of documents mismatch. Expected {}, got {}.\nActual: {:#?}",
        test_case,
        expected.len(),
        actual.len(),
        actual.iter().map(|d| d.page_content()).collect::<Vec<_>>()
    );
    for (idx, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert_eq!(
            a.page_content(),
            e.page_content(),
            "Test Case '{}' Failed at Document {}: Content mismatch.\nExpected: {:?}\nGot: {:?}",
            test_case,
            idx + 1,
            e.page_content(),
            a.page_content()
        );
        assert_eq!(
            a.metadata(),
            e.metadata(),
            "Test Case '{}' Failed at Document {}: Metadata mismatch.\nExpected: {:?}\nGot: {:?}",
            test_case,
            idx + 1,
            e.metadata(),
            a.metadata()
        );
    }
}

// Test Case 1: Split on h1 and h2
#[test]
fn test_html_header_splitter_simple() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![
            ("h1".to_string(), "Header 1".to_string()),
            ("h2".to_string(), "Header 2".to_string()),
        ])
        .return_each_element(true)
        .build();

    let html = r#"
        <html>
            <body>
                <h1>Introduction</h1>
                <p>This is the introduction.</p>
                <h2>Background</h2>
                <p>Background information.</p>
                <h1>Conclusion</h1>
                <p>Final thoughts.</p>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![
        doc("Introduction", vec![("Header 1", "Introduction")]),
        doc(
            "This is the introduction.",
            vec![("Header 1", "Introduction")],
        ),
        doc(
            "Background",
            vec![("Header 1", "Introduction"), ("Header 2", "Background")],
        ),
        doc(
            "Background information.",
            vec![("Header 1", "Introduction"), ("Header 2", "Background")],
        ),
        doc("Conclusion", vec![("Header 1", "Conclusion")]),
        doc("Final thoughts.", vec![("Header 1", "Conclusion")]),
    ];

    assert_docs_eq(&docs, &expected, "Simple headers and paragraphs");
}

// Test Case 2: Nested headers with h1, h2, h3
#[test]
fn test_html_header_splitter_nested() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![
            ("h1".to_string(), "Header 1".to_string()),
            ("h2".to_string(), "Header 2".to_string()),
            ("h3".to_string(), "Header 3".to_string()),
        ])
        .return_each_element(true)
        .build();

    let html = r#"
        <html>
            <body>
                <div>
                    <h1>Main Title</h1>
                    <div>
                        <h2>Subsection</h2>
                        <p>Details of subsection.</p>
                        <div>
                            <h3>Sub-subsection</h3>
                            <p>More details.</p>
                        </div>
                    </div>
                </div>
                <h1>Another Main Title</h1>
                <p>Content under another main title.</p>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![
        doc("Main Title", vec![("Header 1", "Main Title")]),
        doc(
            "Subsection",
            vec![("Header 1", "Main Title"), ("Header 2", "Subsection")],
        ),
        doc(
            "Details of subsection.",
            vec![("Header 1", "Main Title"), ("Header 2", "Subsection")],
        ),
        doc(
            "Sub-subsection",
            vec![
                ("Header 1", "Main Title"),
                ("Header 2", "Subsection"),
                ("Header 3", "Sub-subsection"),
            ],
        ),
        doc(
            "More details.",
            vec![
                ("Header 1", "Main Title"),
                ("Header 2", "Subsection"),
                ("Header 3", "Sub-subsection"),
            ],
        ),
        doc(
            "Another Main Title",
            vec![("Header 1", "Another Main Title")],
        ),
        doc(
            "Content under another main title.",
            vec![("Header 1", "Another Main Title")],
        ),
    ];

    assert_docs_eq(&docs, &expected, "Nested headers with h1, h2, and h3");
}

// Test Case 3: No headers (aggregated)
#[test]
fn test_html_header_splitter_no_headers() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![("h1".to_string(), "Header 1".to_string())])
        .build();

    let html = r#"
        <html>
            <body>
                <p>Paragraph one.</p>
                <p>Paragraph two.</p>
                <div>
                    <p>Paragraph three.</p>
                </div>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![doc(
        "Paragraph one.  \nParagraph two.  \nParagraph three.",
        vec![],
    )];

    assert_docs_eq(&docs, &expected, "No headers present");
}

// Test Case 4: Multiple headers of the same level
#[test]
fn test_html_header_splitter_same_level() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![("h1".to_string(), "Header 1".to_string())])
        .return_each_element(true)
        .build();

    let html = r#"
        <html>
            <body>
                <h1>Chapter 1</h1>
                <p>Content of chapter 1.</p>
                <h1>Chapter 2</h1>
                <p>Content of chapter 2.</p>
                <h1>Chapter 3</h1>
                <p>Content of chapter 3.</p>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![
        doc("Chapter 1", vec![("Header 1", "Chapter 1")]),
        doc("Content of chapter 1.", vec![("Header 1", "Chapter 1")]),
        doc("Chapter 2", vec![("Header 1", "Chapter 2")]),
        doc("Content of chapter 2.", vec![("Header 1", "Chapter 2")]),
        doc("Chapter 3", vec![("Header 1", "Chapter 3")]),
        doc("Content of chapter 3.", vec![("Header 1", "Chapter 3")]),
    ];

    assert_docs_eq(&docs, &expected, "Multiple headers of the same level");
}

// Test Case 5: Headers with no content
#[test]
fn test_html_header_splitter_headers_no_content() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![
            ("h1".to_string(), "Header 1".to_string()),
            ("h2".to_string(), "Header 2".to_string()),
        ])
        .return_each_element(true)
        .build();

    let html = r#"
        <html>
            <body>
                <h1>Header 1</h1>
                <h2>Header 2</h2>
                <h1>Header 3</h1>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![
        doc("Header 1", vec![("Header 1", "Header 1")]),
        doc(
            "Header 2",
            vec![("Header 1", "Header 1"), ("Header 2", "Header 2")],
        ),
        doc("Header 3", vec![("Header 1", "Header 3")]),
    ];

    assert_docs_eq(&docs, &expected, "Headers with no associated content");
}

// Test Case A: Complex nested with h1, h2, h3 (aggregated, return_each_element=false)
#[test]
fn test_html_header_splitter_complex_nested() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![
            ("h1".to_string(), "Header 1".to_string()),
            ("h2".to_string(), "Header 2".to_string()),
            ("h3".to_string(), "Header 3".to_string()),
        ])
        .build();

    let html = r#"
        <!DOCTYPE html>
        <html>
        <body>
            <div>
                <h1>Foo</h1>
                <p>Some intro text about Foo.</p>
                <div>
                    <h2>Bar main section</h2>
                    <p>Some intro text about Bar.</p>
                    <h3>Bar subsection 1</h3>
                    <p>Some text about the first subtopic of Bar.</p>
                    <h3>Bar subsection 2</h3>
                    <p>Some text about the second subtopic of Bar.</p>
                </div>
                <div>
                    <h2>Baz</h2>
                    <p>Some text about Baz</p>
                </div>
                <br>
                <p>Some concluding text about Foo</p>
            </div>
        </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![
        doc("Foo", vec![("Header 1", "Foo")]),
        doc("Some intro text about Foo.", vec![("Header 1", "Foo")]),
        doc(
            "Bar main section",
            vec![("Header 1", "Foo"), ("Header 2", "Bar main section")],
        ),
        doc(
            "Some intro text about Bar.",
            vec![("Header 1", "Foo"), ("Header 2", "Bar main section")],
        ),
        doc(
            "Bar subsection 1",
            vec![
                ("Header 1", "Foo"),
                ("Header 2", "Bar main section"),
                ("Header 3", "Bar subsection 1"),
            ],
        ),
        doc(
            "Some text about the first subtopic of Bar.",
            vec![
                ("Header 1", "Foo"),
                ("Header 2", "Bar main section"),
                ("Header 3", "Bar subsection 1"),
            ],
        ),
        doc(
            "Bar subsection 2",
            vec![
                ("Header 1", "Foo"),
                ("Header 2", "Bar main section"),
                ("Header 3", "Bar subsection 2"),
            ],
        ),
        doc(
            "Some text about the second subtopic of Bar.",
            vec![
                ("Header 1", "Foo"),
                ("Header 2", "Bar main section"),
                ("Header 3", "Bar subsection 2"),
            ],
        ),
        doc("Baz", vec![("Header 1", "Foo"), ("Header 2", "Baz")]),
        doc(
            "Some text about Baz  \nSome concluding text about Foo",
            vec![("Header 1", "Foo")],
        ),
    ];

    assert_docs_eq(&docs, &expected, "Complex nested with h1, h2, h3");
}

// Test Case B: No headers, three paragraphs (aggregated)
#[test]
fn test_html_header_splitter_no_headers_three_paragraphs() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![("h1".to_string(), "Header 1".to_string())])
        .build();

    let html = r#"
        <html>
            <body>
                <p>Paragraph one.</p>
                <p>Paragraph two.</p>
                <p>Paragraph three.</p>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![doc(
        "Paragraph one.  \nParagraph two.  \nParagraph three.",
        vec![],
    )];

    assert_docs_eq(&docs, &expected, "No headers, three paragraphs");
}

// Test Case C: No headers with multiple splitter headers configured
#[test]
fn test_html_no_headers_with_multiple_splitters() {
    let splitter = HTMLHeaderTextSplitter::builder()
        .headers_to_split_on(vec![
            ("h1".to_string(), "Header 1".to_string()),
            ("h2".to_string(), "Header 2".to_string()),
            ("h3".to_string(), "Header 3".to_string()),
        ])
        .build();

    let html = r#"
        <html>
            <body>
                <p>Just some random text without headers.</p>
                <div>
                    <span>More text here.</span>
                </div>
            </body>
        </html>
    "#;

    let docs = splitter.split_text(html);

    let expected = vec![doc(
        "Just some random text without headers.  \nMore text here.",
        vec![],
    )];

    assert_docs_eq(&docs, &expected, "No headers with multiple splitters");
}

// HTMLSectionSplitter tests

#[test]
fn test_section_splitter_header_based() {
    let splitter = HTMLSectionSplitter::new(vec![
        ("h1".to_string(), "Header 1".to_string()),
        ("h2".to_string(), "Header 2".to_string()),
    ]);

    let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div>
                <h1>Foo</h1>
                <p>Some intro text about Foo.</p>
                <div>
                    <h2>Bar main section</h2>
                    <p>Some intro text about Bar.</p>
                    <h3>Bar subsection 1</h3>
                    <p>Some text about the first subtopic of Bar.</p>
                    <h3>Bar subsection 2</h3>
                    <p>Some text about the second subtopic of Bar.</p>
                </div>
                <div>
                    <h2>Baz</h2>
                    <p>Some text about Baz</p>
                </div>
                <br>
                <p>Some concluding text about Foo</p>
            </div>
        </body>
        </html>"#;

    let docs = splitter.split_text(html).unwrap();

    assert_eq!(docs.len(), 3, "Expected 3 documents, got {}", docs.len());
    assert_eq!(docs[0].metadata()["Header 1"], "Foo");
    assert_eq!(docs[1].metadata()["Header 2"], "Bar main section");
    assert_eq!(docs[2].metadata()["Header 2"], "Baz");
}

#[test]
fn test_section_splitter_font_size() {
    let splitter = HTMLSectionSplitter::new(vec![
        ("h1".to_string(), "Header 1".to_string()),
        ("h2".to_string(), "Header 2".to_string()),
    ]);

    let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div>
                <span style="font-size: 22px">Foo</span>
                <p>Some intro text about Foo.</p>
                <div>
                    <h2>Bar main section</h2>
                    <p>Some intro text about Bar.</p>
                </div>
            </div>
        </body>
        </html>"#;

    let docs = splitter.split_text(html).unwrap();

    assert!(
        docs.len() >= 2,
        "Expected at least 2 documents, got {}",
        docs.len()
    );
    assert_eq!(docs[0].metadata()["Header 1"], "Foo");
    assert_eq!(docs[1].metadata()["Header 2"], "Bar main section");
}

#[test]
fn test_section_splitter_font_size_whitespace() {
    let splitter = HTMLSectionSplitter::new(vec![
        ("h1".to_string(), "Header 1".to_string()),
        ("h2".to_string(), "Header 2".to_string()),
    ]);

    let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div>
                <span style="font-size: 22px">
Foo </span>
                <p>Some intro text about Foo.</p>
                <div>
                    <h2>Bar main section</h2>
                    <p>Some intro text about Bar.</p>
                </div>
            </div>
        </body>
        </html>"#;

    let docs = splitter.split_text(html).unwrap();

    assert!(
        docs.len() >= 2,
        "Expected at least 2 documents, got {}",
        docs.len()
    );
    assert_eq!(docs[0].metadata()["Header 1"], "Foo");
}

#[test]
fn test_section_splitter_duplicate_header() {
    let splitter = HTMLSectionSplitter::new(vec![
        ("h1".to_string(), "Header 1".to_string()),
        ("h2".to_string(), "Header 2".to_string()),
    ]);

    let html = r#"<!DOCTYPE html>
        <html>
        <body>
            <div>
                <h1>Foo</h1>
                <p>Some intro text about Foo.</p>
                <div>
                    <h2>Bar main section</h2>
                    <p>Some intro text about Bar.</p>
                </div>
                <div>
                    <h2>Foo</h2>
                    <p>Some text about Baz</p>
                </div>
                <h1>Foo</h1>
                <br>
                <p>Some concluding text about Foo</p>
            </div>
        </body>
        </html>"#;

    let docs = splitter.split_text(html).unwrap();

    assert_eq!(docs.len(), 4, "Expected 4 documents, got {}", docs.len());
    assert_eq!(docs[0].metadata()["Header 1"], "Foo");
    assert_eq!(docs[1].metadata()["Header 2"], "Bar main section");
    assert_eq!(docs[2].metadata()["Header 2"], "Foo");
    assert_eq!(docs[3].metadata()["Header 1"], "Foo");
}

#[test]
fn test_extract_font_size_px() {
    // Test the font-size extraction helper
    let splitter = HTMLSectionSplitter::new(vec![("h1".to_string(), "Header 1".to_string())]);

    // An element with font-size 22px should be converted to h1
    let html =
        r#"<html><body><span style="font-size: 22px">Title</span><p>Body text</p></body></html>"#;
    let docs = splitter.split_text(html).unwrap();
    assert!(!docs.is_empty());
    assert_eq!(docs[0].metadata()["Header 1"], "Title");
}
