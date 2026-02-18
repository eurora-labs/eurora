use std::collections::HashMap;

use agent_chain_core::documents::Document;
use agent_chain_core::embeddings::DeterministicFakeEmbedding;
use agent_chain_core::indexing::{
    CleanupMode, HashAlgorithm, InMemoryDocumentIndex, InMemoryRecordManager, IndexConfig,
    IndexDestination, IndexingResult, KeyEncoder, RecordManager, SourceIdKey, aindex, index,
};
use agent_chain_core::vectorstores::InMemoryVectorStore;
use agent_chain_core::vectorstores::VectorStore;
use serde_json::json;

fn make_record_manager() -> InMemoryRecordManager {
    let manager = InMemoryRecordManager::new("test");
    manager.create_schema().unwrap();
    manager
}

fn make_vector_store() -> InMemoryVectorStore {
    let embeddings = DeterministicFakeEmbedding::new(5);
    InMemoryVectorStore::new(Box::new(embeddings))
}

fn sha256_config() -> IndexConfig {
    IndexConfig {
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    }
}

#[test]
fn test_record_manager_update_and_exists() {
    let manager = make_record_manager();
    let keys = vec!["key1".to_string(), "key2".to_string()];
    manager.update(&keys, None, None).unwrap();

    let exists = manager.exists(&keys).unwrap();
    assert_eq!(exists, vec![true, true]);

    let exists = manager
        .exists(&["key1".to_string(), "missing".to_string()])
        .unwrap();
    assert_eq!(exists, vec![true, false]);
}

#[test]
fn test_record_manager_update_with_group_ids() {
    let manager = make_record_manager();
    manager.set_time_override(Some(1.0));

    let keys = vec!["k1".to_string(), "k2".to_string(), "k3".to_string()];
    let group_ids = vec![
        Some("g1".to_string()),
        Some("g1".to_string()),
        Some("g2".to_string()),
    ];
    manager.update(&keys, Some(&group_ids), None).unwrap();

    let g1_keys = manager
        .list_keys(None, None, Some(&["g1".to_string()]), None)
        .unwrap();
    assert_eq!(g1_keys.len(), 2);

    let g2_keys = manager
        .list_keys(None, None, Some(&["g2".to_string()]), None)
        .unwrap();
    assert_eq!(g2_keys.len(), 1);
    assert_eq!(g2_keys[0], "k3");
}

#[test]
fn test_record_manager_list_keys_filtering() {
    let manager = make_record_manager();

    manager.set_time_override(Some(1.0));
    manager.update(&["k1".to_string()], None, None).unwrap();

    manager.set_time_override(Some(2.0));
    manager.update(&["k2".to_string()], None, None).unwrap();

    manager.set_time_override(Some(3.0));
    manager.update(&["k3".to_string()], None, None).unwrap();

    let before = manager.list_keys(Some(2.0), None, None, None).unwrap();
    assert_eq!(before, vec!["k1".to_string()]);

    let after = manager.list_keys(None, Some(1.0), None, None).unwrap();
    assert_eq!(after, vec!["k2".to_string(), "k3".to_string()]);

    let limited = manager.list_keys(None, None, None, Some(2)).unwrap();
    assert_eq!(limited.len(), 2);
}

#[test]
fn test_record_manager_delete_keys() {
    let manager = make_record_manager();
    let keys = vec!["k1".to_string(), "k2".to_string(), "k3".to_string()];
    manager.update(&keys, None, None).unwrap();

    manager
        .delete_keys(&["k1".to_string(), "k2".to_string()])
        .unwrap();

    let exists = manager.exists(&keys).unwrap();
    assert_eq!(exists, vec![false, false, true]);
}

#[tokio::test]
async fn test_record_manager_async_update_and_exists() {
    let manager = make_record_manager();
    let keys = vec!["key1".to_string(), "key2".to_string()];
    manager.aupdate(&keys, None, None).await.unwrap();

    let exists = manager.aexists(&keys).await.unwrap();
    assert_eq!(exists, vec![true, true]);
}

#[tokio::test]
async fn test_record_manager_async_list_and_delete() {
    let manager = make_record_manager();
    manager.set_time_override(Some(1.0));
    let keys = vec!["k1".to_string(), "k2".to_string()];
    manager.aupdate(&keys, None, None).await.unwrap();

    let listed = manager.alist_keys(None, None, None, None).await.unwrap();
    assert_eq!(listed.len(), 2);

    manager.adelete_keys(&["k1".to_string()]).await.unwrap();
    let listed = manager.alist_keys(None, None, None, None).await.unwrap();
    assert_eq!(listed, vec!["k2".to_string()]);
}

#[test]
fn test_sha1_deterministic_hash() {
    use agent_chain_core::indexing::get_document_with_hash;

    let mut metadata = HashMap::new();
    metadata.insert("key".to_string(), json!("value"));

    let doc = Document {
        page_content: "Lorem ipsum dolor sit amet".to_string(),
        id: None,
        metadata,
        type_: "Document".to_string(),
    };

    let encoder = KeyEncoder::Algorithm(HashAlgorithm::Sha1);
    let hashed = get_document_with_hash(&doc, &encoder).unwrap();

    assert_eq!(
        hashed.id.as_deref(),
        Some("fd1dc827-051b-537d-a1fe-1fa043e8b276")
    );
    assert_eq!(hashed.page_content, doc.page_content);
}

#[test]
fn test_different_algorithms_produce_different_results() {
    use agent_chain_core::indexing::get_document_with_hash;

    let doc = Document::new("test content");

    let sha1_enc = KeyEncoder::Algorithm(HashAlgorithm::Sha1);
    let sha256_enc = KeyEncoder::Algorithm(HashAlgorithm::Sha256);
    let sha512_enc = KeyEncoder::Algorithm(HashAlgorithm::Sha512);
    let blake2_enc = KeyEncoder::Algorithm(HashAlgorithm::Blake2b);

    let h1 = get_document_with_hash(&doc, &sha1_enc).unwrap().id;
    let h2 = get_document_with_hash(&doc, &sha256_enc).unwrap().id;
    let h3 = get_document_with_hash(&doc, &sha512_enc).unwrap().id;
    let h4 = get_document_with_hash(&doc, &blake2_enc).unwrap().id;

    assert_ne!(h1, h2);
    assert_ne!(h2, h3);
    assert_ne!(h3, h4);
}

#[test]
fn test_custom_key_encoder() {
    use agent_chain_core::indexing::get_document_with_hash;

    let doc = Document::new("hello world");
    let encoder = KeyEncoder::Custom(Box::new(|doc: &Document| {
        format!("custom-{}", doc.page_content.len())
    }));

    let hashed = get_document_with_hash(&doc, &encoder).unwrap();
    assert_eq!(hashed.id.as_deref(), Some("custom-11"));
}

#[test]
fn test_indexing_same_content() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);
    let config = sha256_config();

    let docs = vec![
        Document::new("This is a test document."),
        Document::new("This is another document."),
    ];

    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(
        result,
        IndexingResult {
            num_added: 2,
            num_deleted: 0,
            num_skipped: 0,
            num_updated: 0,
        }
    );
    assert_eq!(store.len().unwrap(), 2);

    for _ in 0..2 {
        let result = index(docs.clone(), &manager, &dest, &config).unwrap();
        assert_eq!(
            result,
            IndexingResult {
                num_added: 0,
                num_deleted: 0,
                num_skipped: 2,
                num_updated: 0,
            }
        );
    }
}

#[tokio::test]
async fn test_aindexing_same_content() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);
    let config = sha256_config();

    let docs = vec![
        Document::new("This is a test document."),
        Document::new("This is another document."),
    ];

    let result = aindex(docs.clone(), &manager, &dest, &config)
        .await
        .unwrap();
    assert_eq!(result.num_added, 2);
    assert_eq!(store.len().unwrap(), 2);

    let result = aindex(docs.clone(), &manager, &dest, &config)
        .await
        .unwrap();
    assert_eq!(result.num_added, 0);
    assert_eq!(result.num_skipped, 2);
}

#[test]
fn test_index_simple_delete_full() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);

    let docs = vec![
        Document::new("This is a test document."),
        Document::new("This is another document."),
    ];

    manager.set_time_override(Some(1609459200.0)); // 2021-01-01
    let config = IndexConfig {
        cleanup: Some(CleanupMode::Full),
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 2);

    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_skipped, 2);
    assert_eq!(result.num_deleted, 0);

    let docs2 = vec![
        Document::new("mutated document 1"),
        Document::new("This is another document."),
    ];

    manager.set_time_override(Some(1609545600.0)); // 2021-01-02
    let result = index(docs2.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 1);
    assert_eq!(result.num_skipped, 1);
    assert_eq!(result.num_deleted, 1);

    let store_keys = store.store_keys().unwrap();
    assert_eq!(store_keys.len(), 2);

    let all_docs = store.get_by_ids(&store_keys).unwrap();
    let texts: std::collections::HashSet<String> =
        all_docs.into_iter().map(|d| d.page_content).collect();
    assert!(texts.contains("mutated document 1"));
    assert!(texts.contains("This is another document."));

    let result = index(docs2, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 0);
    assert_eq!(result.num_deleted, 0);
    assert_eq!(result.num_skipped, 2);
}

#[test]
fn test_incremental_fails_with_bad_source_ids() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);

    let config = IndexConfig {
        cleanup: Some(CleanupMode::Incremental),
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(vec![Document::new("test")], &manager, &dest, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Source id key is required"));

    let docs = vec![
        Document::new("test").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("test2").with_metadata(HashMap::from([("source".to_string(), json!(null))])),
    ];
    let config = IndexConfig {
        cleanup: Some(CleanupMode::Incremental),
        source_id_key: Some(SourceIdKey::MetadataKey("source".to_string())),
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(docs, &manager, &dest, &config);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Source IDs are required"));
}

#[test]
fn test_index_simple_delete_scoped_full() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);

    let docs = vec![
        Document::new("doc1").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("doc2").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("doc3").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("doc_other")
            .with_metadata(HashMap::from([("source".to_string(), json!("2"))])),
    ];

    manager.set_time_override(Some(1.0));
    let config = IndexConfig {
        cleanup: Some(CleanupMode::ScopedFull),
        source_id_key: Some(SourceIdKey::MetadataKey("source".to_string())),
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 4);

    manager.set_time_override(Some(2.0));
    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_skipped, 4);

    let docs2 = vec![
        Document::new("mutated doc")
            .with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("doc2").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
    ];
    manager.set_time_override(Some(3.0));
    let result = index(docs2.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 1);
    assert_eq!(result.num_skipped, 1);
    assert_eq!(result.num_deleted, 2); // doc1 and doc3 from source=1 deleted

    assert_eq!(store.len().unwrap(), 3); // mutated_doc + doc2 + doc_other

    manager.set_time_override(Some(4.0));
    let result = index(docs2, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 0);
    assert_eq!(result.num_skipped, 2);
    assert_eq!(result.num_deleted, 0);
}

#[test]
fn test_indexing_with_no_docs() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);
    let config = sha256_config();

    let result = index(Vec::<Document>::new(), &manager, &dest, &config).unwrap();
    assert_eq!(
        result,
        IndexingResult {
            num_added: 0,
            num_deleted: 0,
            num_skipped: 0,
            num_updated: 0,
        }
    );
}

#[test]
fn test_deduplication() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);
    let config = sha256_config();

    let docs = vec![
        Document::new("duplicate content"),
        Document::new("duplicate content"),
        Document::new("unique content"),
    ];

    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 2);
    assert_eq!(result.num_skipped, 1); // one duplicate skipped
    assert_eq!(store.len().unwrap(), 2);
}

#[test]
fn test_indexing_force_update() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);

    let docs = vec![Document::new("some content")];

    let config = sha256_config();
    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 1);

    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_skipped, 1);

    let config = IndexConfig {
        force_update: true,
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_updated, 1);
    assert_eq!(result.num_added, 0);
}

#[test]
fn test_index_into_document_index() {
    let manager = make_record_manager();
    let doc_index = InMemoryDocumentIndex::default();
    let dest = IndexDestination::DocumentIndex(&doc_index);
    let config = sha256_config();

    let docs = vec![Document::new("doc one"), Document::new("doc two")];

    let result = index(docs.clone(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 2);
    assert_eq!(doc_index.len().unwrap(), 2);

    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 0);
    assert_eq!(result.num_skipped, 2);
}

#[test]
fn test_document_index_upsert_and_get() {
    use agent_chain_core::indexing::DocumentIndex;

    let index = InMemoryDocumentIndex::default();
    let docs = vec![
        Document::new("hello world").with_id("id1"),
        Document::new("foo bar"),
    ];

    let response = index.upsert(&docs).unwrap();
    assert_eq!(response.succeeded.len(), 2);
    assert_eq!(response.succeeded[0], "id1");
    assert!(response.failed.is_empty());

    let retrieved = index.get(&["id1".to_string()]).unwrap();
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].page_content, "hello world");

    let retrieved = index.get(&[response.succeeded[1].clone()]).unwrap();
    assert_eq!(retrieved.len(), 1);
    assert_eq!(retrieved[0].page_content, "foo bar");
}

#[test]
fn test_document_index_delete() {
    use agent_chain_core::indexing::DocumentIndex;

    let index = InMemoryDocumentIndex::default();
    let docs = vec![
        Document::new("a").with_id("1"),
        Document::new("b").with_id("2"),
        Document::new("c").with_id("3"),
    ];
    index.upsert(&docs).unwrap();

    let response = index
        .delete(Some(&["1".to_string(), "2".to_string()]))
        .unwrap();
    assert_eq!(response.num_deleted, Some(2));
    assert_eq!(index.len().unwrap(), 1);

    let remaining = index.get(&["3".to_string()]).unwrap();
    assert_eq!(remaining.len(), 1);
}

#[test]
fn test_document_index_retriever_ordering() {
    use agent_chain_core::indexing::DocumentIndex;
    use agent_chain_core::retrievers::BaseRetriever;

    let idx = InMemoryDocumentIndex::new(2);
    let docs = vec![
        Document::new("the cat sat on the mat").with_id("1"),
        Document::new("the the the the the").with_id("2"),
        Document::new("dog walks in park").with_id("3"),
    ];
    idx.upsert(&docs).unwrap();

    let results = idx.invoke("the", None).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].page_content, "the the the the the");
    assert_eq!(results[1].page_content, "the cat sat on the mat");
}

#[test]
fn test_scoped_full_empty_loader() {
    let manager = make_record_manager();
    let store = make_vector_store();
    let dest = IndexDestination::VectorStore(&store);

    let docs = vec![
        Document::new("doc1").with_metadata(HashMap::from([("source".to_string(), json!("1"))])),
        Document::new("doc2").with_metadata(HashMap::from([("source".to_string(), json!("2"))])),
    ];

    manager.set_time_override(Some(1.0));
    let config = IndexConfig {
        cleanup: Some(CleanupMode::ScopedFull),
        source_id_key: Some(SourceIdKey::MetadataKey("source".to_string())),
        key_encoder: KeyEncoder::Algorithm(HashAlgorithm::Sha256),
        ..Default::default()
    };
    let result = index(docs, &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 2);

    manager.set_time_override(Some(2.0));
    let result = index(Vec::<Document>::new(), &manager, &dest, &config).unwrap();
    assert_eq!(result.num_added, 0);
    assert_eq!(result.num_deleted, 0);
    assert_eq!(store.len().unwrap(), 2);
}
