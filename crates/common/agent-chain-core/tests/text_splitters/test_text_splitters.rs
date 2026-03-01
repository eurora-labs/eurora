use std::collections::HashMap;

use agent_chain_core::documents::Document;
use agent_chain_core::text_splitters::base::merge_splits;
use agent_chain_core::{
    CharacterTextSplitter, CharacterTextSplitterConfig, KeepSeparator, Language,
    PythonCodeTextSplitter, RecursiveCharacterTextSplitter, TextSplitter, TextSplitterConfig,
};

// ---------------------------------------------------------------------------
// Character text splitter tests
// ---------------------------------------------------------------------------

#[test]
fn test_character_text_splitter() {
    let config = TextSplitterConfig::new(7, 3, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("foo bar baz 123").unwrap();
    assert_eq!(output, vec!["foo bar", "bar baz", "baz 123"]);
}

#[test]
fn test_character_text_splitter_empty_doc() {
    let config = TextSplitterConfig::new(2, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("foo  bar").unwrap();
    assert_eq!(output, vec!["foo", "bar"]);
}

#[test]
fn test_character_text_splitter_separator_empty_doc() {
    let config = TextSplitterConfig::new(2, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("f b").unwrap();
    assert_eq!(output, vec!["f", "b"]);
}

#[test]
fn test_character_text_splitter_long() {
    let config = TextSplitterConfig::new(3, 1, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("foo bar baz a a").unwrap();
    assert_eq!(output, vec!["foo", "bar", "baz", "a a"]);
}

#[test]
fn test_character_text_splitter_short_words_first() {
    let config = TextSplitterConfig::new(3, 1, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("a a foo bar baz").unwrap();
    assert_eq!(output, vec!["a a", "foo", "bar", "baz"]);
}

#[test]
fn test_character_text_splitter_longer_words() {
    let config = TextSplitterConfig::new(1, 1, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let output = splitter.split_text("foo bar baz 123").unwrap();
    assert_eq!(output, vec!["foo", "bar", "baz", "123"]);
}

#[test]
fn test_character_text_splitter_keep_separator_regex() {
    // Test with regex separator (escaped dot)
    let config =
        TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"\.".to_string(),
            is_separator_regex: true,
        },
        config,
    );
    let output = splitter.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output, vec!["foo", ".bar", ".baz", ".123"]);

    // Test with literal separator
    let config2 =
        TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter2 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: ".".to_string(),
            is_separator_regex: false,
        },
        config2,
    );
    let output2 = splitter2.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output2, vec!["foo", ".bar", ".baz", ".123"]);
}

#[test]
fn test_character_text_splitter_keep_separator_regex_end() {
    // Test with regex separator
    let config = TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::End), None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"\.".to_string(),
            is_separator_regex: true,
        },
        config,
    );
    let output = splitter.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output, vec!["foo.", "bar.", "baz.", "123"]);

    // Test with literal separator
    let config2 =
        TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::End), None, None).unwrap();
    let splitter2 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: ".".to_string(),
            is_separator_regex: false,
        },
        config2,
    );
    let output2 = splitter2.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output2, vec!["foo.", "bar.", "baz.", "123"]);
}

#[test]
fn test_character_text_splitter_discard_separator_regex() {
    // Test with regex separator
    let config =
        TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"\.".to_string(),
            is_separator_regex: true,
        },
        config,
    );
    let output = splitter.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output, vec!["foo", "bar", "baz", "123"]);

    // Test with literal separator
    let config2 =
        TextSplitterConfig::new(1, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let splitter2 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: ".".to_string(),
            is_separator_regex: false,
        },
        config2,
    );
    let output2 = splitter2.split_text("foo.bar.baz.123").unwrap();
    assert_eq!(output2, vec!["foo", "bar", "baz", "123"]);
}

#[test]
fn test_character_text_splitter_discard_regex_separator_on_merge() {
    let config =
        TextSplitterConfig::new(200, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"(?=SCE\d{3})".to_string(),
            is_separator_regex: true,
        },
        config,
    );
    let output = splitter
        .split_text("SCE191 First chunk. SCE103 Second chunk.")
        .unwrap();
    assert_eq!(output, vec!["SCE191 First chunk. SCE103 Second chunk."]);
}

#[test]
fn test_character_text_splitter_chunk_size_effect() {
    // regex lookbehind & split happens
    let config1 =
        TextSplitterConfig::new(5, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let s1 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"(?<=mid)".to_string(),
            is_separator_regex: true,
        },
        config1,
    );
    assert_eq!(s1.split_text("abcmiddef").unwrap(), vec!["abcmid", "def"]);

    // regex lookbehind & no split (chunk_size large enough to merge)
    let config2 =
        TextSplitterConfig::new(100, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let s2 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: r"(?<=mid)".to_string(),
            is_separator_regex: true,
        },
        config2,
    );
    assert_eq!(s2.split_text("abcmiddef").unwrap(), vec!["abcmiddef"]);

    // literal separator & split happens
    let config3 =
        TextSplitterConfig::new(3, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let s3 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: "mid".to_string(),
            is_separator_regex: false,
        },
        config3,
    );
    assert_eq!(s3.split_text("abcmiddef").unwrap(), vec!["abc", "def"]);

    // literal separator & no split
    let config4 =
        TextSplitterConfig::new(100, 0, None, Some(KeepSeparator::None), None, None).unwrap();
    let s4 = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: "mid".to_string(),
            is_separator_regex: false,
        },
        config4,
    );
    assert_eq!(s4.split_text("abcmiddef").unwrap(), vec!["abcmiddef"]);
}

// ---------------------------------------------------------------------------
// Recursive character text splitter tests
// ---------------------------------------------------------------------------

#[test]
fn test_recursive_character_text_splitter_keep_separators() {
    let split_tags = vec![",".to_string(), ".".to_string()];
    let query = "Apple,banana,orange and tomato.";

    // start
    let config =
        TextSplitterConfig::new(10, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::builder()
        .separators(split_tags.clone())
        .config(config)
        .build();
    let result = splitter.split_text(query).unwrap();
    assert_eq!(result, vec!["Apple", ",banana", ",orange and tomato", "."]);

    // end
    let config2 =
        TextSplitterConfig::new(10, 0, None, Some(KeepSeparator::End), None, None).unwrap();
    let splitter2 = RecursiveCharacterTextSplitter::builder()
        .separators(split_tags)
        .config(config2)
        .build();
    let result2 = splitter2.split_text(query).unwrap();
    assert_eq!(result2, vec!["Apple,", "banana,", "orange and tomato."]);
}

#[test]
fn test_character_text_splitting_args() {
    // chunk_overlap > chunk_size
    assert!(TextSplitterConfig::new(2, 4, None, None, None, None).is_err());
    // chunk_size == 0
    assert!(TextSplitterConfig::new(0, 0, None, None, None, None).is_err());
    // chunk_overlap negative (usize can't be negative, so we just test 0 chunk_size)
}

#[test]
fn test_merge_splits_fn() {
    let config = TextSplitterConfig::new(9, 2, None, None, None, None).unwrap();
    let splits = vec!["foo".to_string(), "bar".to_string(), "baz".to_string()];
    let output = merge_splits(&splits, " ", &config);
    assert_eq!(output, vec!["foo bar", "baz"]);
}

// ---------------------------------------------------------------------------
// Document creation tests
// ---------------------------------------------------------------------------

#[test]
fn test_create_documents() {
    let config = TextSplitterConfig::new(3, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let texts = vec!["foo bar".to_string(), "baz".to_string()];
    let docs = splitter.create_documents(&texts, None).unwrap();

    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].page_content, "foo");
    assert_eq!(docs[1].page_content, "bar");
    assert_eq!(docs[2].page_content, "baz");
}

#[test]
fn test_create_documents_with_metadata() {
    let config = TextSplitterConfig::new(3, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let texts = vec!["foo bar".to_string(), "baz".to_string()];
    let metadatas = vec![
        HashMap::from([("source".to_string(), serde_json::json!("1"))]),
        HashMap::from([("source".to_string(), serde_json::json!("2"))]),
    ];
    let docs = splitter.create_documents(&texts, Some(&metadatas)).unwrap();

    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].page_content, "foo");
    assert_eq!(docs[0].metadata["source"], "1");
    assert_eq!(docs[1].page_content, "bar");
    assert_eq!(docs[1].metadata["source"], "1");
    assert_eq!(docs[2].page_content, "baz");
    assert_eq!(docs[2].metadata["source"], "2");
}

#[test]
fn test_create_documents_with_start_index_character() {
    let config = TextSplitterConfig::new(7, 3, None, None, Some(true), None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let text = "foo bar baz 123";
    let docs = splitter
        .create_documents(&[text.to_string()], None)
        .unwrap();

    assert_eq!(docs.len(), 3);
    assert_eq!(docs[0].page_content, "foo bar");
    assert_eq!(docs[0].metadata["start_index"], 0);
    assert_eq!(docs[1].page_content, "bar baz");
    assert_eq!(docs[1].metadata["start_index"], 4);
    assert_eq!(docs[2].page_content, "baz 123");
    assert_eq!(docs[2].metadata["start_index"], 8);

    for doc in &docs {
        let si = doc.metadata["start_index"].as_u64().unwrap() as usize;
        assert_eq!(&text[si..si + doc.page_content.len()], doc.page_content);
    }
}

#[test]
fn test_create_documents_with_start_index_recursive() {
    let config = TextSplitterConfig::new(6, 0, None, None, Some(true), None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::builder()
        .separators(vec![
            "\n\n".to_string(),
            "\n".to_string(),
            " ".to_string(),
            String::new(),
        ])
        .config(config)
        .build();
    let text = "w1 w1 w1 w1 w1 w1 w1 w1 w1";
    let docs = splitter
        .create_documents(&[text.to_string()], None)
        .unwrap();

    assert_eq!(docs.len(), 5);
    assert_eq!(docs[0].page_content, "w1 w1");
    assert_eq!(docs[0].metadata["start_index"], 0);
    assert_eq!(docs[1].page_content, "w1 w1");
    assert_eq!(docs[1].metadata["start_index"], 6);
    assert_eq!(docs[2].page_content, "w1 w1");
    assert_eq!(docs[2].metadata["start_index"], 12);
    assert_eq!(docs[3].page_content, "w1 w1");
    assert_eq!(docs[3].metadata["start_index"], 18);
    assert_eq!(docs[4].page_content, "w1");
    assert_eq!(docs[4].metadata["start_index"], 24);

    for doc in &docs {
        let si = doc.metadata["start_index"].as_u64().unwrap() as usize;
        assert_eq!(&text[si..si + doc.page_content.len()], doc.page_content);
    }
}

#[test]
fn test_metadata_not_shallow() {
    let config = TextSplitterConfig::new(3, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: " ".to_string(),
            is_separator_regex: false,
        },
        config,
    );
    let texts = vec!["foo bar".to_string()];
    let metadatas = vec![HashMap::from([(
        "source".to_string(),
        serde_json::json!("1"),
    )])];
    let mut docs = splitter.create_documents(&texts, Some(&metadatas)).unwrap();

    assert_eq!(docs.len(), 2);
    docs[0]
        .metadata
        .insert("foo".to_string(), serde_json::json!(1));
    assert_eq!(docs[0].metadata["source"], "1");
    assert_eq!(docs[0].metadata["foo"], 1);
    assert!(!docs[1].metadata.contains_key("foo"));
    assert_eq!(docs[1].metadata["source"], "1");
}

// ---------------------------------------------------------------------------
// Iterative text splitter tests
// ---------------------------------------------------------------------------

fn run_iterative_text_splitter(chunk_size: usize, keep_separator: bool) -> Vec<String> {
    let effective_chunk_size = if keep_separator {
        chunk_size + 1
    } else {
        chunk_size
    };
    let keep = if keep_separator {
        KeepSeparator::Start
    } else {
        KeepSeparator::None
    };
    let config =
        TextSplitterConfig::new(effective_chunk_size, 0, None, Some(keep), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::builder()
        .separators(vec!["X".to_string(), "Y".to_string()])
        .config(config)
        .build();
    let text = "....5X..3Y...4X....5Y...";
    let output = splitter.split_text(text).unwrap();
    for chunk in &output {
        assert!(
            chunk.len() <= effective_chunk_size,
            "Chunk is larger than {}",
            effective_chunk_size
        );
    }
    output
}

#[test]
fn test_iterative_text_splitter_keep_separator() {
    let output = run_iterative_text_splitter(5, true);
    assert_eq!(output, vec!["....5", "X..3", "Y...4", "X....5", "Y..."]);
}

#[test]
fn test_iterative_text_splitter_discard_separator() {
    let output = run_iterative_text_splitter(5, false);
    assert_eq!(output, vec!["....5", "..3", "...4", "....5", "..."]);
}

#[test]
fn test_iterative_text_splitter() {
    let text = "Hi.\n\nI'm Harrison.\n\nHow? Are? You?\nOkay then f f f f.\nThis is a weird text to write, but gotta test the splittingggg some how.\n\nBye!\n\n-H.";
    let config =
        TextSplitterConfig::new(10, 1, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::builder()
        .config(config)
        .build();
    let output = splitter.split_text(text).unwrap();
    let expected = vec![
        "Hi.",
        "I'm",
        "Harrison.",
        "How? Are?",
        "You?",
        "Okay then",
        "f f f f.",
        "This is a",
        "weird",
        "text to",
        "write,",
        "but gotta",
        "test the",
        "splitting",
        "gggg",
        "some how.",
        "Bye!",
        "-H.",
    ];
    assert_eq!(output, expected);
}

// ---------------------------------------------------------------------------
// split_documents test
// ---------------------------------------------------------------------------

#[test]
fn test_split_documents() {
    let config = TextSplitterConfig::new(1, 0, None, None, None, None).unwrap();
    let splitter = CharacterTextSplitter::new(
        CharacterTextSplitterConfig {
            separator: String::new(),
            is_separator_regex: false,
        },
        config,
    );
    let docs = vec![
        Document::builder()
            .page_content("foo")
            .metadata(HashMap::from([(
                "source".to_string(),
                serde_json::json!("1"),
            )]))
            .build(),
        Document::builder()
            .page_content("bar")
            .metadata(HashMap::from([(
                "source".to_string(),
                serde_json::json!("2"),
            )]))
            .build(),
        Document::builder()
            .page_content("baz")
            .metadata(HashMap::from([(
                "source".to_string(),
                serde_json::json!("1"),
            )]))
            .build(),
    ];
    let result = splitter.split_documents(&docs).unwrap();
    assert_eq!(result.len(), 9);
    assert_eq!(result[0].page_content, "f");
    assert_eq!(result[0].metadata["source"], "1");
    assert_eq!(result[3].page_content, "b");
    assert_eq!(result[3].metadata["source"], "2");
    assert_eq!(result[6].page_content, "b");
    assert_eq!(result[6].metadata["source"], "1");
}

// ---------------------------------------------------------------------------
// PythonCodeTextSplitter test
// ---------------------------------------------------------------------------

#[test]
fn test_python_text_splitter() {
    let config =
        TextSplitterConfig::new(30, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = PythonCodeTextSplitter::new(config);
    let fake_python =
        "\nclass Foo:\n\n    def bar():\n\n\ndef foo():\n\ndef testing_func():\n\ndef bar():\n";
    let splits = splitter.split_text(fake_python).unwrap();
    assert_eq!(
        splits,
        vec![
            "class Foo:\n\n    def bar():",
            "def foo():",
            "def testing_func():",
            "def bar():",
        ]
    );
}

// ---------------------------------------------------------------------------
// Code language splitter tests (from_language)
// ---------------------------------------------------------------------------

const CHUNK_SIZE: usize = 16;

#[test]
fn test_python_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Python, config);
    let code = "\ndef hello_world():\n    print(\"Hello, World!\")\n\n# Call the function\nhello_world()\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "def",
            "hello_world():",
            "print(\"Hello,",
            "World!\")",
            "# Call the",
            "function",
            "hello_world()",
        ]
    );
}

#[test]
fn test_golang_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Go, config);
    let code = "\npackage main\n\nimport \"fmt\"\n\nfunc helloWorld() {\n    fmt.Println(\"Hello, World!\")\n}\n\nfunc main() {\n    helloWorld()\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "package main",
            "import \"fmt\"",
            "func",
            "helloWorld() {",
            "fmt.Println(\"He",
            "llo,",
            "World!\")",
            "}",
            "func main() {",
            "helloWorld()",
            "}",
        ]
    );
}

#[test]
fn test_rst_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Rst, config);
    let code = "\nSample Document\n===============\n\nSection\n-------\n\nThis is the content of the section.\n\nLists\n-----\n\n- Item 1\n- Item 2\n- Item 3\n\nComment\n*******\nNot a comment\n\n.. This is a comment\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "Sample Document",
            "===============",
            "Section",
            "-------",
            "This is the",
            "content of the",
            "section.",
            "Lists",
            "-----",
            "- Item 1",
            "- Item 2",
            "- Item 3",
            "Comment",
            "*******",
            "Not a comment",
            ".. This is a",
            "comment",
        ]
    );

    // Special test for special characters
    let code2 = "harry\n***\nbabylon is";
    let chunks2 = splitter.split_text(code2).unwrap();
    assert_eq!(chunks2, vec!["harry", "***\nbabylon is"]);
}

#[test]
fn test_proto_file_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Proto, config);
    let code = "\nsyntax = \"proto3\";\n\npackage example;\n\nmessage Person {\n    string name = 1;\n    int32 age = 2;\n    repeated string hobbies = 3;\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "syntax =",
            "\"proto3\";",
            "package",
            "example;",
            "message Person",
            "{",
            "string name",
            "= 1;",
            "int32 age =",
            "2;",
            "repeated",
            "string hobbies",
            "= 3;",
            "}",
        ]
    );
}

#[test]
fn test_javascript_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Js, config);
    let code = "\nfunction helloWorld() {\n  console.log(\"Hello, World!\");\n}\n\n// Call the function\nhelloWorld();\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "function",
            "helloWorld() {",
            "console.log(\"He",
            "llo,",
            "World!\");",
            "}",
            "// Call the",
            "function",
            "helloWorld();",
        ]
    );
}

#[test]
fn test_cobol_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Cobol, config);
    let code = "\nIDENTIFICATION DIVISION.\nPROGRAM-ID. HelloWorld.\nDATA DIVISION.\nWORKING-STORAGE SECTION.\n01 GREETING           PIC X(12)   VALUE 'Hello, World!'.\nPROCEDURE DIVISION.\nDISPLAY GREETING.\nSTOP RUN.\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "IDENTIFICATION",
            "DIVISION.",
            "PROGRAM-ID.",
            "HelloWorld.",
            "DATA DIVISION.",
            "WORKING-STORAGE",
            "SECTION.",
            "01 GREETING",
            "PIC X(12)",
            "VALUE 'Hello,",
            "World!'.",
            "PROCEDURE",
            "DIVISION.",
            "DISPLAY",
            "GREETING.",
            "STOP RUN.",
        ]
    );
}

#[test]
fn test_typescript_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Ts, config);
    let code = "\nfunction helloWorld(): void {\n  console.log(\"Hello, World!\");\n}\n\n// Call the function\nhelloWorld();\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "function",
            "helloWorld():",
            "void {",
            "console.log(\"He",
            "llo,",
            "World!\");",
            "}",
            "// Call the",
            "function",
            "helloWorld();",
        ]
    );
}

#[test]
fn test_java_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Java, config);
    let code = "\npublic class HelloWorld {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, World!\");\n    }\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "public class",
            "HelloWorld {",
            "public",
            "static void",
            "main(String[]",
            "args) {",
            "System.out.prin",
            "tln(\"Hello,",
            "World!\");",
            "}\n}",
        ]
    );
}

#[test]
fn test_kotlin_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Kotlin, config);
    let code = "\nclass HelloWorld {\n    companion object {\n        @JvmStatic\n        fun main(args: Array<String>) {\n            println(\"Hello, World!\")\n        }\n    }\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "class",
            "HelloWorld {",
            "companion",
            "object {",
            "@JvmStatic",
            "fun",
            "main(args:",
            "Array<String>)",
            "{",
            "println(\"Hello,",
            "World!\")",
            "}\n    }",
            "}",
        ]
    );
}

#[test]
fn test_csharp_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::CSharp, config);
    let code = "\nusing System;\nclass Program\n{\n    static void Main()\n    {\n        int age = 30; // Change the age value as needed\n\n        // Categorize the age without any console output\n        if (age < 18)\n        {\n            // Age is under 18\n        }\n        else if (age >= 18 && age < 65)\n        {\n            // Age is an adult\n        }\n        else\n        {\n            // Age is a senior citizen\n        }\n    }\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "using System;",
            "class Program\n{",
            "static void",
            "Main()",
            "{",
            "int age",
            "= 30; // Change",
            "the age value",
            "as needed",
            "//",
            "Categorize the",
            "age without any",
            "console output",
            "if (age",
            "< 18)",
            "{",
            "//",
            "Age is under 18",
            "}",
            "else if",
            "(age >= 18 &&",
            "age < 65)",
            "{",
            "//",
            "Age is an adult",
            "}",
            "else",
            "{",
            "//",
            "Age is a senior",
            "citizen",
            "}\n    }",
            "}",
        ]
    );
}

#[test]
fn test_cpp_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Cpp, config);
    let code = "\n#include <iostream>\n\nint main() {\n    std::cout << \"Hello, World!\" << std::endl;\n    return 0;\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "#include",
            "<iostream>",
            "int main() {",
            "std::cout",
            "<< \"Hello,",
            "World!\" <<",
            "std::endl;",
            "return 0;\n}",
        ]
    );
}

#[test]
fn test_scala_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Scala, config);
    let code = "\nobject HelloWorld {\n  def main(args: Array[String]): Unit = {\n    println(\"Hello, World!\")\n  }\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "object",
            "HelloWorld {",
            "def",
            "main(args:",
            "Array[String]):",
            "Unit = {",
            "println(\"Hello,",
            "World!\")",
            "}\n}",
        ]
    );
}

#[test]
fn test_ruby_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Ruby, config);
    let code = "\ndef hello_world\n  puts \"Hello, World!\"\nend\n\nhello_world\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "def hello_world",
            "puts \"Hello,",
            "World!\"",
            "end",
            "hello_world",
        ]
    );
}

#[test]
fn test_php_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Php, config);
    let code = "\n<?php\nfunction hello_world() {\n    echo \"Hello, World!\";\n}\n\nhello_world();\n?>\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "<?php",
            "function",
            "hello_world() {",
            "echo",
            "\"Hello,",
            "World!\";",
            "}",
            "hello_world();",
            "?>",
        ]
    );
}

#[test]
fn test_swift_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Swift, config);
    let code = "\nfunc helloWorld() {\n    print(\"Hello, World!\")\n}\n\nhelloWorld()\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "func",
            "helloWorld() {",
            "print(\"Hello,",
            "World!\")",
            "}",
            "helloWorld()",
        ]
    );
}

#[test]
fn test_rust_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Rust, config);
    let code = "\nfn main() {\n    println!(\"Hello, World!\");\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec!["fn main() {", "println!(\"Hello", ",", "World!\");", "}"]
    );
}

#[test]
fn test_r_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::R, config);
    let code = "\nlibrary(dplyr)\n\nmy_func <- function(x) {\n    return(x + 1)\n}\n\nif (TRUE) {\n    print(\"Hello\")\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "library(dplyr)",
            "my_func <-",
            "function(x) {",
            "return(x +",
            "1)",
            "}",
            "if (TRUE) {",
            "print(\"Hello\")",
            "}",
        ]
    );
}

#[test]
fn test_markdown_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Markdown, config);
    let code = "\n# Sample Document\n\n## Section\n\nThis is the content of the section.\n\n## Lists\n\n- Item 1\n- Item 2\n- Item 3\n\n### Horizontal lines\n\n***********\n____________\n-------------------\n\n#### Code blocks\n```\nThis is a code block\n\n# sample code\na = 1\nb = 2\n```\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "# Sample",
            "Document",
            "## Section",
            "This is the",
            "content of the",
            "section.",
            "## Lists",
            "- Item 1",
            "- Item 2",
            "- Item 3",
            "### Horizontal",
            "lines",
            "***********",
            "____________",
            "---------------",
            "----",
            "#### Code",
            "blocks",
            "```",
            "This is a code",
            "block",
            "# sample code",
            "a = 1\nb = 2",
            "```",
        ]
    );

    // Special test for special characters
    let code2 = "harry\n***\nbabylon is";
    let chunks2 = splitter.split_text(code2).unwrap();
    assert_eq!(chunks2, vec!["harry", "***\nbabylon is"]);
}

#[test]
fn test_latex_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Latex, config);
    let code = "\nHi Harrison!\n\\chapter{1}\n";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(chunks, vec!["Hi Harrison!", "\\chapter{1}"]);
}

#[test]
fn test_html_code_splitter() {
    let config =
        TextSplitterConfig::new(60, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Html, config);
    let code = "\n<h1>Sample Document</h1>\n    <h2>Section</h2>\n        <p id=\"1234\">Reference content.</p>\n\n    <h2>Lists</h2>\n        <ul>\n            <li>Item 1</li>\n            <li>Item 2</li>\n            <li>Item 3</li>\n        </ul>\n\n        <h3>A block</h3>\n            <div class=\"amazing\">\n                <p>Some text</p>\n                <p>Some more text</p>\n            </div>\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "<h1>Sample Document</h1>\n    <h2>Section</h2>",
            "<p id=\"1234\">Reference content.</p>",
            "<h2>Lists</h2>\n        <ul>",
            "<li>Item 1</li>\n            <li>Item 2</li>",
            "<li>Item 3</li>\n        </ul>",
            "<h3>A block</h3>",
            "<div class=\"amazing\">",
            "<p>Some text</p>",
            "<p>Some more text</p>\n            </div>",
        ]
    );
}

#[test]
fn test_solidity_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Sol, config);
    let code = "pragma solidity ^0.8.20;\n  contract HelloWorld {\n    function add(uint a, uint b) pure public returns(uint) {\n      return  a + b;\n    }\n  }\n  ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "pragma solidity",
            "^0.8.20;",
            "contract",
            "HelloWorld {",
            "function",
            "add(uint a,",
            "uint b) pure",
            "public",
            "returns(uint) {",
            "return  a",
            "+ b;",
            "}\n  }",
        ]
    );
}

#[test]
fn test_lua_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Lua, config);
    let code = "\nlocal variable = 10\n\nfunction add(a, b)\n    return a + b\nend\n\nif variable > 5 then\n    for i=1, variable do\n        while i < variable do\n            repeat\n                print(i)\n                i = i + 1\n            until i >= variable\n        end\n    end\nend\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "local variable",
            "= 10",
            "function add(a,",
            "b)",
            "return a +",
            "b",
            "end",
            "if variable > 5",
            "then",
            "for i=1,",
            "variable do",
            "while i",
            "< variable do",
            "repeat",
            "print(i)",
            "i = i + 1",
            "until i >=",
            "variable",
            "end",
            "end\nend",
        ]
    );
}

#[test]
fn test_haskell_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::Haskell, config);
    let code = "\n        main :: IO ()\n        main = do\n          putStrLn \"Hello, World!\"\n\n        -- Some sample functions\n        add :: Int -> Int -> Int\n        add x y = x + y\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "main ::",
            "IO ()",
            "main = do",
            "putStrLn",
            "\"Hello, World!\"",
            "--",
            "Some sample",
            "functions",
            "add :: Int ->",
            "Int -> Int",
            "add x y = x",
            "+ y",
        ]
    );
}

#[test]
fn test_powershell_code_splitter_short_code() {
    let config =
        TextSplitterConfig::new(60, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::PowerShell, config);
    let code = "\n# Check if a file exists\n$filePath = \"C:\\temp\\file.txt\"\nif (Test-Path $filePath) {\n    # File exists\n} else {\n    # File does not exist\n}\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "# Check if a file exists\n$filePath = \"C:\\temp\\file.txt\"",
            "if (Test-Path $filePath) {\n    # File exists\n} else {",
            "# File does not exist\n}",
        ]
    );
}

#[test]
fn test_powershell_code_splitter_longer_code() {
    let config =
        TextSplitterConfig::new(60, 0, None, Some(KeepSeparator::Start), None, None).unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::PowerShell, config);
    let code = "\n# Get a list of all processes and export to CSV\n$processes = Get-Process\n$processes | Export-Csv -Path \"C:\\temp\\processes.csv\" -NoTypeInformation\n\n# Read the CSV file and display its content\n$csvContent = Import-Csv -Path \"C:\\temp\\processes.csv\"\n$csvContent | ForEach-Object {\n    $_.ProcessName\n}\n\n# End of script\n    ";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "# Get a list of all processes and export to CSV",
            "$processes = Get-Process",
            "$processes | Export-Csv -Path \"C:\\temp\\processes.csv\"",
            "-NoTypeInformation",
            "# Read the CSV file and display its content",
            "$csvContent = Import-Csv -Path \"C:\\temp\\processes.csv\"",
            "$csvContent | ForEach-Object {\n    $_.ProcessName\n}",
            "# End of script",
        ]
    );
}

#[test]
fn test_visualbasic6_code_splitter() {
    let config =
        TextSplitterConfig::new(CHUNK_SIZE, 0, None, Some(KeepSeparator::Start), None, None)
            .unwrap();
    let splitter = RecursiveCharacterTextSplitter::from_language(Language::VisualBasic6, config);
    let code = "\nOption Explicit\n\nPublic Function SumTwoIntegers(ByVal a As Integer, ByVal b As Integer) As Integer\n    SumTwoIntegers = a + b\nEnd Function\n\nPublic Sub Main()\n    Dim i As Integer\n    Dim limit As Integer\n\n    i = 0\n    limit = 50\n\n    While i < limit\n        i = SumTwoIntegers(i, 1)\n\n        If i = limit \\ 2 Then\n            MsgBox \"Halfway there! i = \" & i\n        End If\n    Wend\n\n    MsgBox \"Done! Final value of i: \" & i\nEnd Sub\n";
    let chunks = splitter.split_text(code).unwrap();
    assert_eq!(
        chunks,
        vec![
            "Option Explicit",
            "Public Function",
            "SumTwoIntegers(",
            "ByVal",
            "a As Integer,",
            "ByVal b As",
            "Integer) As",
            "Integer",
            "SumTwoIntegers",
            "= a + b",
            "End Function",
            "Public Sub",
            "Main()",
            "Dim i As",
            "Integer",
            "Dim limit",
            "As Integer",
            "i = 0",
            "limit = 50",
            "While i <",
            "limit",
            "i =",
            "SumTwoIntegers(",
            "i,",
            "1)",
            "If i =",
            "limit \\ 2 Then",
            "MsgBox \"Halfway",
            "there! i = \" &",
            "i",
            "End If",
            "Wend",
            "MsgBox",
            "\"Done! Final",
            "value of i: \" &",
            "i",
            "End Sub",
        ]
    );
}
