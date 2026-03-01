use std::collections::HashMap;

use agent_chain_core::documents::Document;
use agent_chain_core::{ExperimentalMarkdownSyntaxTextSplitter, MarkdownHeaderTextSplitter};

fn doc(content: &str, metadata: Vec<(&str, &str)>) -> Document {
    let m: HashMap<String, serde_json::Value> = metadata
        .into_iter()
        .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
        .collect();
    Document::builder()
        .page_content(content)
        .metadata(m)
        .build()
}

// ---------------------------------------------------------------------------
// MarkdownHeaderTextSplitter tests
// ---------------------------------------------------------------------------

#[test]
fn test_md_header_text_splitter_1() {
    let markdown_document =
        "# Foo\n\n    ## Bar\n\nHi this is Jim\n\nHi this is Joe\n\n ## Baz\n\n Hi this is Molly";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "Hi this is Jim  \nHi this is Joe",
                vec![("Header 1", "Foo"), ("Header 2", "Bar")]
            ),
            doc(
                "Hi this is Molly",
                vec![("Header 1", "Foo"), ("Header 2", "Baz")]
            ),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_2() {
    let markdown_document = "# Foo\n\n    ## Bar\n\nHi this is Jim\n\nHi this is Joe\n\n ### Boo \n\n Hi this is Lance \n\n ## Baz\n\n Hi this is Molly";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
        ("###".to_string(), "Header 3".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "Hi this is Jim  \nHi this is Joe",
                vec![("Header 1", "Foo"), ("Header 2", "Bar")]
            ),
            doc(
                "Hi this is Lance",
                vec![
                    ("Header 1", "Foo"),
                    ("Header 2", "Bar"),
                    ("Header 3", "Boo")
                ]
            ),
            doc(
                "Hi this is Molly",
                vec![("Header 1", "Foo"), ("Header 2", "Baz")]
            ),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_3() {
    let markdown_document = "# Foo\n\n    ## Bar\n\nHi this is Jim\n\nHi this is Joe\n\n ### Boo \n\n Hi this is Lance \n\n #### Bim \n\n Hi this is John \n\n ## Baz\n\n Hi this is Molly";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
        ("###".to_string(), "Header 3".to_string()),
        ("####".to_string(), "Header 4".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "Hi this is Jim  \nHi this is Joe",
                vec![("Header 1", "Foo"), ("Header 2", "Bar")]
            ),
            doc(
                "Hi this is Lance",
                vec![
                    ("Header 1", "Foo"),
                    ("Header 2", "Bar"),
                    ("Header 3", "Boo")
                ]
            ),
            doc(
                "Hi this is John",
                vec![
                    ("Header 1", "Foo"),
                    ("Header 2", "Bar"),
                    ("Header 3", "Boo"),
                    ("Header 4", "Bim")
                ]
            ),
            doc(
                "Hi this is Molly",
                vec![("Header 1", "Foo"), ("Header 2", "Baz")]
            ),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_preserve_headers_1() {
    let markdown_document = "# Foo\n\n    ## Bat\n\nHi this is Jim\n\nHi Joe\n\n## Baz\n\n# Bar\n\nThis is Alice\n\nThis is Bob";
    let headers = vec![("#".to_string(), "Header 1".to_string())];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, false, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "# Foo  \n## Bat  \nHi this is Jim  \nHi Joe  \n## Baz",
                vec![("Header 1", "Foo")]
            ),
            doc(
                "# Bar  \nThis is Alice  \nThis is Bob",
                vec![("Header 1", "Bar")]
            ),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_preserve_headers_2() {
    let markdown_document = "# Foo\n\n    ## Bar\n\nHi this is Jim\n\nHi this is Joe\n\n### Boo \n\nHi this is Lance\n\n## Baz\n\nHi this is Molly\n    ## Buz\n# Bop";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
        ("###".to_string(), "Header 3".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, false, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "# Foo  \n## Bar  \nHi this is Jim  \nHi this is Joe",
                vec![("Header 1", "Foo"), ("Header 2", "Bar")]
            ),
            doc(
                "### Boo  \nHi this is Lance",
                vec![
                    ("Header 1", "Foo"),
                    ("Header 2", "Bar"),
                    ("Header 3", "Boo")
                ]
            ),
            doc(
                "## Baz  \nHi this is Molly",
                vec![("Header 1", "Foo"), ("Header 2", "Baz")]
            ),
            doc("## Buz", vec![("Header 1", "Foo"), ("Header 2", "Buz")]),
            doc("# Bop", vec![("Header 1", "Bop")]),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_fenced_code_block_backticks() {
    let markdown_document = "# This is a Header\n\n```\nfoo()\n# Not a header\nbar()\n```";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![doc(
            "```\nfoo()\n# Not a header\nbar()\n```",
            vec![("Header 1", "This is a Header")]
        ),]
    );
}

#[test]
fn test_md_header_text_splitter_fenced_code_block_tildes() {
    let markdown_document = "# This is a Header\n\n~~~\nfoo()\n# Not a header\nbar()\n~~~";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![doc(
            "~~~\nfoo()\n# Not a header\nbar()\n~~~",
            vec![("Header 1", "This is a Header")]
        ),]
    );
}

#[test]
fn test_md_header_text_splitter_fenced_code_block_interleaved_backticks_tildes() {
    let markdown_document =
        "# This is a Header\n\n```\nfoo\n# Not a header\n~~~\n# Not a header\n```";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![doc(
            "```\nfoo\n# Not a header\n~~~\n# Not a header\n```",
            vec![("Header 1", "This is a Header")]
        ),]
    );
}

#[test]
fn test_md_header_text_splitter_fenced_code_block_interleaved_tildes_backticks() {
    let markdown_document =
        "# This is a Header\n\n~~~\nfoo\n# Not a header\n```\n# Not a header\n~~~";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![doc(
            "~~~\nfoo\n# Not a header\n```\n# Not a header\n~~~",
            vec![("Header 1", "This is a Header")]
        ),]
    );
}

#[test]
fn test_md_header_text_splitter_with_invisible_characters() {
    let markdown_document = "\u{feff}# Foo\n\nfoo()\n\u{feff}## Bar\n\nbar()";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
    ];
    let splitter = MarkdownHeaderTextSplitter::new(headers, false, true, None);
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc("foo()", vec![("Header 1", "Foo")]),
            doc("bar()", vec![("Header 1", "Foo"), ("Header 2", "Bar")]),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_with_custom_headers() {
    let markdown_document = "**Chapter 1**\n\nThis is the content for chapter 1.\n\n***Section 1.1***\n\nThis is the content for section 1.1.\n\n**Chapter 2**\n\nThis is the content for chapter 2.\n\n***Section 2.1***\n\nThis is the content for section 2.1.\n";
    let headers = vec![
        ("**".to_string(), "Bold Header".to_string()),
        ("***".to_string(), "Bold Italic Header".to_string()),
    ];
    let custom_header_patterns =
        HashMap::from([("**".to_string(), 1usize), ("***".to_string(), 2usize)]);
    let splitter =
        MarkdownHeaderTextSplitter::new(headers, false, true, Some(custom_header_patterns));
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "This is the content for chapter 1.",
                vec![("Bold Header", "Chapter 1")]
            ),
            doc(
                "This is the content for section 1.1.",
                vec![
                    ("Bold Header", "Chapter 1"),
                    ("Bold Italic Header", "Section 1.1")
                ]
            ),
            doc(
                "This is the content for chapter 2.",
                vec![("Bold Header", "Chapter 2")]
            ),
            doc(
                "This is the content for section 2.1.",
                vec![
                    ("Bold Header", "Chapter 2"),
                    ("Bold Italic Header", "Section 2.1")
                ]
            ),
        ]
    );
}

#[test]
fn test_md_header_text_splitter_mixed_headers() {
    let markdown_document = "# Standard Header 1\n\nContent under standard header.\n\n**Custom Header 1**\n\nContent under custom header.\n\n## Standard Header 2\n\nContent under standard header 2.\n\n***Custom Header 2***\n\nContent under custom header 2.\n";
    let headers = vec![
        ("#".to_string(), "Header 1".to_string()),
        ("##".to_string(), "Header 2".to_string()),
        ("**".to_string(), "Bold Header".to_string()),
        ("***".to_string(), "Bold Italic Header".to_string()),
    ];
    let custom_header_patterns =
        HashMap::from([("**".to_string(), 1usize), ("***".to_string(), 2usize)]);
    let splitter =
        MarkdownHeaderTextSplitter::new(headers, false, true, Some(custom_header_patterns));
    let output = splitter.split_text(markdown_document).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "Content under standard header.",
                vec![("Header 1", "Standard Header 1")]
            ),
            doc(
                "Content under custom header.",
                vec![("Bold Header", "Custom Header 1")]
            ),
            doc(
                "Content under standard header 2.",
                vec![
                    ("Bold Header", "Custom Header 1"),
                    ("Header 2", "Standard Header 2")
                ]
            ),
            doc(
                "Content under custom header 2.",
                vec![
                    ("Bold Header", "Custom Header 1"),
                    ("Bold Italic Header", "Custom Header 2")
                ]
            ),
        ]
    );
}

// ---------------------------------------------------------------------------
// ExperimentalMarkdownSyntaxTextSplitter tests
// ---------------------------------------------------------------------------

const EXPERIMENTAL_MARKDOWN_DOCUMENT: &str = "# My Header 1\nContent for header 1\n## Header 2\nContent for header 2\n### Header 3\nContent for header 3\n## Header 2 Again\nThis should be tagged with Header 1 and Header 2 Again\n```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n# Header 1 again\nWe should also split on the horizontal line\n----\nThis will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph";

#[test]
fn test_experimental_markdown_syntax_text_splitter() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, true);
    let output = splitter.split_text(EXPERIMENTAL_MARKDOWN_DOCUMENT).unwrap();

    assert_eq!(
        output,
        vec![
            doc("Content for header 1\n", vec![("Header 1", "My Header 1")]),
            doc(
                "Content for header 2\n",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2")]
            ),
            doc(
                "Content for header 3\n",
                vec![
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2"),
                    ("Header 3", "Header 3")
                ]
            ),
            doc(
                "This should be tagged with Header 1 and Header 2 Again\n",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2 Again")]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_header_configuration() {
    let headers = vec![("#".to_string(), "Encabezamiento 1".to_string())];
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(Some(headers), false, true);
    let output = splitter.split_text(EXPERIMENTAL_MARKDOWN_DOCUMENT).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "Content for header 1\n## Header 2\nContent for header 2\n### Header 3\nContent for header 3\n## Header 2 Again\nThis should be tagged with Header 1 and Header 2 Again\n",
                vec![("Encabezamiento 1", "My Header 1")]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![("Code", "python"), ("Encabezamiento 1", "My Header 1")]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Encabezamiento 1", "Header 1 again")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Encabezamiento 1", "Header 1 again")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_with_headers() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, false);
    let output = splitter.split_text(EXPERIMENTAL_MARKDOWN_DOCUMENT).unwrap();

    assert_eq!(
        output,
        vec![
            doc(
                "# My Header 1\nContent for header 1\n",
                vec![("Header 1", "My Header 1")]
            ),
            doc(
                "## Header 2\nContent for header 2\n",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2")]
            ),
            doc(
                "### Header 3\nContent for header 3\n",
                vec![
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2"),
                    ("Header 3", "Header 3")
                ]
            ),
            doc(
                "## Header 2 Again\nThis should be tagged with Header 1 and Header 2 Again\n",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2 Again")]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "# Header 1 again\nWe should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_split_lines() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, true, true);
    let output = splitter.split_text(EXPERIMENTAL_MARKDOWN_DOCUMENT).unwrap();

    assert_eq!(
        output,
        vec![
            doc("Content for header 1", vec![("Header 1", "My Header 1")]),
            doc(
                "Content for header 2",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2")]
            ),
            doc(
                "Content for header 3",
                vec![
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2"),
                    ("Header 3", "Header 3")
                ]
            ),
            doc(
                "This should be tagged with Header 1 and Header 2 Again",
                vec![("Header 1", "My Header 1"), ("Header 2", "Header 2 Again")]
            ),
            doc(
                "```python",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "def func_definition():",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "   print('Keep the whitespace consistent')",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "```",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1"),
                    ("Header 2", "Header 2 Again")
                ]
            ),
            doc(
                "We should also split on the horizontal line",
                vec![("Header 1", "Header 1 again")]
            ),
            doc(
                "This will be a new doc but with the same header metadata",
                vec![("Header 1", "Header 1 again")]
            ),
            doc(
                "And it includes a new paragraph",
                vec![("Header 1", "Header 1 again")]
            ),
        ]
    );
}

// ---------------------------------------------------------------------------
// Multi-file tests
// ---------------------------------------------------------------------------

fn experimental_markdown_documents() -> Vec<String> {
    vec![
        "# My Header 1 From Document 1\nContent for header 1 from Document 1\n## Header 2 From Document 1\nContent for header 2 from Document 1\n```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n# Header 1 again From Document 1\nWe should also split on the horizontal line\n----\nThis will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph".to_string(),
        "# My Header 1 From Document 2\nContent for header 1 from Document 2\n## Header 2 From Document 2\nContent for header 2 from Document 2\n```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n# Header 1 again From Document 2\nWe should also split on the horizontal line\n----\nThis will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph".to_string(),
    ]
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_on_multi_files() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, true);
    let mut output = Vec::new();
    for document in experimental_markdown_documents() {
        output.extend(splitter.split_text(&document).unwrap());
    }

    assert_eq!(
        output,
        vec![
            doc(
                "Content for header 1 from Document 1\n",
                vec![("Header 1", "My Header 1 From Document 1")]
            ),
            doc(
                "Content for header 2 from Document 1\n",
                vec![
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "Content for header 1 from Document 2\n",
                vec![("Header 1", "My Header 1 From Document 2")]
            ),
            doc(
                "Content for header 2 from Document 2\n",
                vec![
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_split_lines_on_multi_files() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, true, true);
    let mut output = Vec::new();
    for document in experimental_markdown_documents() {
        output.extend(splitter.split_text(&document).unwrap());
    }

    assert_eq!(
        output,
        vec![
            doc(
                "Content for header 1 from Document 1",
                vec![("Header 1", "My Header 1 From Document 1")]
            ),
            doc(
                "Content for header 2 from Document 1",
                vec![
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "```python",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "def func_definition():",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "   print('Keep the whitespace consistent')",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "```",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "We should also split on the horizontal line",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "This will be a new doc but with the same header metadata",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "And it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "Content for header 1 from Document 2",
                vec![("Header 1", "My Header 1 From Document 2")]
            ),
            doc(
                "Content for header 2 from Document 2",
                vec![
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "```python",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "def func_definition():",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "   print('Keep the whitespace consistent')",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "```",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "We should also split on the horizontal line",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
            doc(
                "This will be a new doc but with the same header metadata",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
            doc(
                "And it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_with_header_on_multi_files() {
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(None, false, false);
    let mut output = Vec::new();
    for document in experimental_markdown_documents() {
        output.extend(splitter.split_text(&document).unwrap());
    }

    assert_eq!(
        output,
        vec![
            doc(
                "# My Header 1 From Document 1\nContent for header 1 from Document 1\n",
                vec![("Header 1", "My Header 1 From Document 1")]
            ),
            doc(
                "## Header 2 From Document 1\nContent for header 2 from Document 1\n",
                vec![
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 1"),
                    ("Header 2", "Header 2 From Document 1")
                ]
            ),
            doc(
                "# Header 1 again From Document 1\nWe should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 1")]
            ),
            doc(
                "# My Header 1 From Document 2\nContent for header 1 from Document 2\n",
                vec![("Header 1", "My Header 1 From Document 2")]
            ),
            doc(
                "## Header 2 From Document 2\nContent for header 2 from Document 2\n",
                vec![
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Header 1", "My Header 1 From Document 2"),
                    ("Header 2", "Header 2 From Document 2")
                ]
            ),
            doc(
                "# Header 1 again From Document 2\nWe should also split on the horizontal line\n",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Header 1", "Header 1 again From Document 2")]
            ),
        ]
    );
}

#[test]
fn test_experimental_markdown_syntax_text_splitter_header_config_on_multi_files() {
    let headers = vec![("#".to_string(), "Encabezamiento 1".to_string())];
    let splitter = ExperimentalMarkdownSyntaxTextSplitter::new(Some(headers), false, true);
    let mut output = Vec::new();
    for document in experimental_markdown_documents() {
        output.extend(splitter.split_text(&document).unwrap());
    }

    assert_eq!(
        output,
        vec![
            doc(
                "Content for header 1 from Document 1\n## Header 2 From Document 1\nContent for header 2 from Document 1\n",
                vec![("Encabezamiento 1", "My Header 1 From Document 1")]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Encabezamiento 1", "My Header 1 From Document 1")
                ]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Encabezamiento 1", "Header 1 again From Document 1")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Encabezamiento 1", "Header 1 again From Document 1")]
            ),
            doc(
                "Content for header 1 from Document 2\n## Header 2 From Document 2\nContent for header 2 from Document 2\n",
                vec![("Encabezamiento 1", "My Header 1 From Document 2")]
            ),
            doc(
                "```python\ndef func_definition():\n   print('Keep the whitespace consistent')\n```\n",
                vec![
                    ("Code", "python"),
                    ("Encabezamiento 1", "My Header 1 From Document 2")
                ]
            ),
            doc(
                "We should also split on the horizontal line\n",
                vec![("Encabezamiento 1", "Header 1 again From Document 2")]
            ),
            doc(
                "This will be a new doc but with the same header metadata\n\nAnd it includes a new paragraph",
                vec![("Encabezamiento 1", "Header 1 again From Document 2")]
            ),
        ]
    );
}
