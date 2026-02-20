use agent_chain_core::utils::function_calling::remove_titles;
use serde_json::json;

#[test]
fn test_rm_titles_schema1() {
    let schema = json!({
        "type": "object",
        "properties": {
            "people": {
                "title": "People",
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "title": "Person",
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "name": {"title": "Name", "type": "string"},
                        "title": {
                            "title": "Title",
                            "description": "person's age",
                            "type": "integer"
                        }
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["people"]
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "people": {
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "title": {"description": "person's age", "type": "integer"}
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["people"]
    });

    assert_eq!(remove_titles(&schema), expected);
}

#[test]
fn test_rm_titles_schema2() {
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {
                "title": "Title",
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "title": "Person",
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "name": {"title": "Name", "type": "string"},
                        "age": {
                            "title": "Age",
                            "description": "person's age",
                            "type": "integer"
                        }
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["title"]
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "title": {
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"},
                        "age": {"description": "person's age", "type": "integer"}
                    },
                    "required": ["name"]
                }
            }
        },
        "required": ["title"]
    });

    assert_eq!(remove_titles(&schema), expected);
}

#[test]
fn test_rm_titles_schema3() {
    let schema = json!({
        "type": "object",
        "properties": {
            "title": {
                "title": "Title",
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "title": "Person",
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "title": {"title": "Title", "type": "string"},
                        "type": {
                            "title": "Type",
                            "description": "person's age",
                            "type": "integer"
                        }
                    },
                    "required": ["title"]
                }
            }
        },
        "required": ["title"]
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "title": {
                "description": "List of info about people",
                "type": "array",
                "items": {
                    "description": "Information about a person.",
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "type": {"description": "person's age", "type": "integer"}
                    },
                    "required": ["title"]
                }
            }
        },
        "required": ["title"]
    });

    assert_eq!(remove_titles(&schema), expected);
}

#[test]
fn test_rm_titles_schema4() {
    let schema = json!({
        "type": "object",
        "properties": {
            "properties": {
                "title": "Info",
                "description": "Information to extract",
                "type": "object",
                "properties": {
                    "title": {
                        "title": "Paper",
                        "description": "Information about papers mentioned.",
                        "type": "object",
                        "properties": {
                            "title": {"title": "Title", "type": "string"},
                            "author": {"title": "Author", "type": "string"}
                        },
                        "required": ["title"]
                    }
                },
                "required": ["title"]
            }
        },
        "required": ["properties"]
    });

    let expected = json!({
        "type": "object",
        "properties": {
            "properties": {
                "description": "Information to extract",
                "type": "object",
                "properties": {
                    "title": {
                        "description": "Information about papers mentioned.",
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"},
                            "author": {"type": "string"}
                        },
                        "required": ["title"]
                    }
                },
                "required": ["title"]
            }
        },
        "required": ["properties"]
    });

    assert_eq!(remove_titles(&schema), expected);
}

#[test]
fn test_rm_titles_schema5() {
    let schema = json!({
        "description": "A list of data.",
        "items": {
            "description": "foo",
            "properties": {
                "title": {"type": "string", "description": "item title"},
                "due_date": {"type": "string", "description": "item due date"}
            },
            "required": [],
            "type": "object"
        },
        "type": "array"
    });

    let expected = json!({
        "description": "A list of data.",
        "items": {
            "description": "foo",
            "properties": {
                "title": {"type": "string", "description": "item title"},
                "due_date": {"type": "string", "description": "item due date"}
            },
            "required": [],
            "type": "object"
        },
        "type": "array"
    });

    assert_eq!(remove_titles(&schema), expected);
}
