//! Message module for LangGraph workflows.
//!
//! This module provides message-related utilities and types.

use std::collections::HashMap;

pub use agent_chain_core::messages::HasId;

/// Merges two lists of messages, updating existing messages by ID.
///
/// By default, this ensures the state is "append-only", unless the
/// new message has the same ID as an existing message.
///
/// # Arguments
///
/// * `left` - The base list of messages.
/// * `right` - The list of messages to merge into the base list.
///
/// # Returns
///
/// A new list of messages with the messages from `right` merged into `left`.
/// If a message in `right` has the same ID as a message in `left`, the
/// message from `right` will replace the message from `left`.
///
/// # Example
///
/// ```ignore
/// use agent_graph::graph::add_messages;
///
/// let messages1 = vec![HumanMessage::new("Hello").with_id("1")];
/// let messages2 = vec![AIMessage::new("Hi there!").with_id("2")];
/// let merged = add_messages(messages1, messages2);
/// // merged contains both messages
/// ```
pub fn add_messages<T: Clone + HasId>(mut left: Vec<T>, right: Vec<T>) -> Vec<T> {
    let mut id_to_idx: HashMap<String, usize> = HashMap::new();
    for (idx, msg) in left.iter().enumerate() {
        if let Some(id) = msg.get_id() {
            id_to_idx.insert(id.to_string(), idx);
        }
    }

    for msg in right {
        if let Some(id) = msg.get_id() {
            if let Some(&existing_idx) = id_to_idx.get(&id) {
                left[existing_idx] = msg;
            } else {
                id_to_idx.insert(id.to_string(), left.len());
                left.push(msg);
            }
        } else {
            left.push(msg);
        }
    }

    left
}

/// A state schema with a messages field, similar to langgraph's MessagesState.
///
/// This trait can be implemented by state types that have a messages field
/// that should use the `add_messages` reducer.
pub trait MessagesState {
    /// The message type used in the state.
    type Message: Clone + HasId;

    /// Get the messages from the state.
    fn messages(&self) -> &Vec<Self::Message>;

    /// Get mutable reference to the messages.
    fn messages_mut(&mut self) -> &mut Vec<Self::Message>;

    /// Update messages using the add_messages reducer.
    fn update_messages(&mut self, new_messages: Vec<Self::Message>) {
        let current = std::mem::take(self.messages_mut());
        *self.messages_mut() = add_messages(current, new_messages);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestMessage {
        id: Option<String>,
        content: String,
    }

    impl HasId for TestMessage {
        fn get_id(&self) -> Option<String> {
            self.id.clone()
        }
    }

    #[test]
    fn test_add_messages_append() {
        let left = vec![TestMessage {
            id: Some("1".to_string()),
            content: "Hello".to_string(),
        }];
        let right = vec![TestMessage {
            id: Some("2".to_string()),
            content: "World".to_string(),
        }];

        let result = add_messages(left, right);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "Hello");
        assert_eq!(result[1].content, "World");
    }

    #[test]
    fn test_add_messages_update() {
        let left = vec![TestMessage {
            id: Some("1".to_string()),
            content: "Hello".to_string(),
        }];
        let right = vec![TestMessage {
            id: Some("1".to_string()),
            content: "Updated".to_string(),
        }];

        let result = add_messages(left, right);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].content, "Updated");
    }

    #[test]
    fn test_add_messages_no_id() {
        let left = vec![TestMessage {
            id: None,
            content: "Hello".to_string(),
        }];
        let right = vec![TestMessage {
            id: None,
            content: "World".to_string(),
        }];

        let result = add_messages(left, right);
        assert_eq!(result.len(), 2);
    }
}
