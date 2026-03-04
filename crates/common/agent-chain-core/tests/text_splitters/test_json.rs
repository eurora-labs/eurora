use serde_json::json;

use agent_chain_core::RecursiveJsonSplitter;

#[test]
fn test_split_json() {
    let max_chunk = 800;
    let splitter = RecursiveJsonSplitter::builder()
        .max_chunk_size(max_chunk)
        .build();

    let mut val1 = serde_json::Map::new();
    for i in 0..100 {
        val1.insert(format!("val1{}", i), json!("testvalue"));
    }
    let mut val16 = serde_json::Map::new();
    for i in 0..100 {
        val16.insert(format!("val16{}", i), json!("testvalue"));
    }
    val1.insert("val16".to_string(), json!(val16));

    let test_data = json!({
        "val0": "testvalue",
        "val1": val1,
    });

    let docs = splitter
        .create_documents(&[test_data], false, None)
        .unwrap();
    for doc in &docs {
        assert!(
            doc.page_content().len() < (max_chunk as f64 * 1.05) as usize,
            "Chunk too large: {} > {}",
            doc.page_content().len(),
            (max_chunk as f64 * 1.05) as usize,
        );
    }
    assert!(!docs.is_empty());
}

#[test]
fn test_split_json_with_lists() {
    let max_chunk = 800;
    let splitter = RecursiveJsonSplitter::builder()
        .max_chunk_size(max_chunk)
        .build();

    let mut val1 = serde_json::Map::new();
    for i in 0..100 {
        val1.insert(format!("val1{}", i), json!("testvalue"));
    }
    let mut val16 = serde_json::Map::new();
    for i in 0..100 {
        val16.insert(format!("val16{}", i), json!("testvalue"));
    }
    val1.insert("val16".to_string(), json!(val16));

    let test_data = json!({
        "val0": "testvalue",
        "val1": val1,
    });

    let test_data_list = json!({
        "testPreprocessing": [test_data.clone()],
    });

    let texts = splitter.split_text(&test_data, false).unwrap();
    let texts_list = splitter.split_text(&test_data_list, true).unwrap();

    assert!(texts_list.len() >= texts.len());
}

#[test]
fn test_split_json_many_calls() {
    let splitter = RecursiveJsonSplitter::builder()
        .max_chunk_size(2000)
        .build();

    let x = json!({"a": 1, "b": 2});
    let y = json!({"c": 3, "d": 4});

    let chunk0 = splitter.split_json(&x, false).unwrap();
    assert_eq!(chunk0, vec![json!({"a": 1, "b": 2})]);

    let chunk1 = splitter.split_json(&y, false).unwrap();
    assert_eq!(chunk1, vec![json!({"c": 3, "d": 4})]);

    // Verify chunk0 is not altered by creating chunk1
    assert_eq!(chunk0, vec![json!({"a": 1, "b": 2})]);
}
