//! Adapters between database rows and the [`thread_core`] HTTP wire types.
//!
//! Three jobs:
//!
//! 1. Translate `be_remote_db::Thread` rows into [`thread_core::Thread`].
//! 2. Translate `be_remote_db::Message` rows into [`AnyMessage`] (carried
//!    typed by [`thread_core::MessageNode::message`]).
//! 3. Build [`MessageNode`] trees from the two row shapes the database
//!    layer hands us (`BranchMessageRow` for active-branch+siblings views,
//!    flat `Message` for the all-variants view).

use std::collections::HashMap;

use agent_chain::{AIMessage, AnyMessage, HumanMessage, SystemMessage, ToolCall, ToolMessage};
use agent_chain_core::messages::ContentBlocks;
use be_remote_db::{BranchMessageRow, Message, MessageType, Thread as DbThread};
use serde_json::Value;
use thread_core::{MessageNode, Thread as WireThread};
use uuid::Uuid;

use crate::error::{ThreadServiceError, ThreadServiceResult};

/// Convert a database thread row into the wire-facing [`thread_core::Thread`].
///
/// `title` defaults to an empty string if the row has none — the gRPC
/// contract this replaces did the same, and the desktop client expects a
/// string (not nullable). Title-less threads are just freshly created ones
/// that haven't run their `generate_title` step yet.
pub fn db_thread_to_wire(thread: DbThread) -> WireThread {
    WireThread {
        id: thread.id,
        user_id: thread.user_id,
        title: thread.title.unwrap_or_default(),
        created_at: thread.created_at,
        updated_at: thread.updated_at,
        active_leaf_id: thread.active_leaf_id,
    }
}

/// Convert a stored `Message` row into the matching [`AnyMessage`] variant.
///
/// AI rows additionally hydrate their `tool_calls` from the JSON column on
/// disk; tool rows require a `tool_call_id`. Rows missing required pieces
/// surface as `Internal` errors — we never want a corrupt row to silently
/// degrade a message's meaning at chat-context-prep time.
pub fn convert_db_message_to_base_message(db_message: Message) -> ThreadServiceResult<AnyMessage> {
    let id = db_message.id.to_string();
    let content = parse_content_blocks(db_message.content);

    match db_message.message_type {
        MessageType::Human => {
            let message = HumanMessage::builder().id(id).content(content).build();
            Ok(AnyMessage::HumanMessage(message))
        }
        MessageType::System => {
            let message = SystemMessage::builder().id(id).content(content).build();
            Ok(AnyMessage::SystemMessage(message))
        }
        MessageType::Ai => {
            let tool_calls = parse_tool_calls(&db_message.tool_calls)?;
            let message = AIMessage::builder()
                .id(id)
                .content(content)
                .tool_calls(tool_calls)
                .build();
            Ok(AnyMessage::AIMessage(message))
        }
        MessageType::Tool => {
            let tool_call_id = db_message.tool_call_id.ok_or_else(|| {
                ThreadServiceError::Internal("Tool message missing tool_call_id".to_string())
            })?;
            let message = ToolMessage::builder()
                .id(id)
                .content(content)
                .tool_call_id(tool_call_id)
                .build();
            Ok(AnyMessage::ToolMessage(message))
        }
    }
}

fn parse_content_blocks(value: Value) -> ContentBlocks {
    serde_json::from_value(value).unwrap_or_default()
}

fn parse_tool_calls(tool_calls: &Option<Value>) -> ThreadServiceResult<Vec<ToolCall>> {
    match tool_calls {
        None => Ok(Vec::new()),
        Some(Value::Null) => Ok(Vec::new()),
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|e| ThreadServiceError::Internal(format!("Failed to parse tool calls: {e}"))),
    }
}

/// Build the active-branch-with-siblings view for the message tree.
///
/// `rows` are the `BranchMessageRow`s the DB returns: each row carries a
/// concrete message plus the id of the *active* sibling at its branch level
/// (`branch_message_id`). We collapse rows into one [`MessageNode`] per
/// branch level, where `message` is the active sibling and `children` is the
/// full set of alternatives (including the active one). The resulting `Vec`
/// is the spine of the active branch ordered by depth.
pub fn build_branch_tree(rows: Vec<BranchMessageRow>) -> ThreadServiceResult<Vec<MessageNode>> {
    struct Group {
        parent_id: Option<Uuid>,
        depth: i32,
        active_index: i32,
        active_message: Option<AnyMessage>,
        children: Vec<MessageNode>,
    }

    let mut groups: Vec<Group> = Vec::new();
    let mut current_branch_id: Option<Uuid> = None;

    for row in rows {
        let is_active = row.message.id == row.branch_message_id;
        let parent_id = row.message.parent_message_id;
        let depth = row.branch_depth;
        let sibling_index = row.sibling_index as i32;
        let message = convert_db_message_to_base_message(row.message)?;

        let sibling = MessageNode {
            parent_id,
            message: message.clone(),
            children: vec![],
            sibling_index,
            depth,
        };

        if current_branch_id != Some(row.branch_message_id) {
            current_branch_id = Some(row.branch_message_id);
            groups.push(Group {
                parent_id,
                depth,
                active_index: 0,
                active_message: None,
                children: Vec::new(),
            });
        }

        let group = groups
            .last_mut()
            .expect("group always exists after push above");
        if is_active {
            group.active_index = group.children.len() as i32;
            group.active_message = Some(message);
        }
        group.children.push(sibling);
    }

    groups
        .into_iter()
        .map(|g| {
            let message = g.active_message.ok_or_else(|| {
                ThreadServiceError::Internal(
                    "Branch group missing active sibling message".to_string(),
                )
            })?;
            Ok(MessageNode {
                parent_id: g.parent_id,
                message,
                children: g.children,
                sibling_index: g.active_index,
                depth: g.depth,
            })
        })
        .collect()
}

/// Build the full message tree (every variant of every branch) from a flat
/// list of database rows. Used for the `all_variants=true` view.
pub fn build_full_tree(messages: Vec<Message>) -> ThreadServiceResult<Vec<MessageNode>> {
    let mut children_by_parent: HashMap<Option<Uuid>, Vec<Message>> = HashMap::new();
    for msg in messages {
        children_by_parent
            .entry(msg.parent_message_id)
            .or_default()
            .push(msg);
    }
    build_subtree(None, &children_by_parent, 0)
}

fn build_subtree(
    parent_id: Option<Uuid>,
    children_by_parent: &HashMap<Option<Uuid>, Vec<Message>>,
    depth: i32,
) -> ThreadServiceResult<Vec<MessageNode>> {
    let Some(siblings) = children_by_parent.get(&parent_id) else {
        return Ok(vec![]);
    };
    siblings
        .iter()
        .enumerate()
        .map(|(idx, msg)| {
            let children = build_subtree(Some(msg.id), children_by_parent, depth + 1)?;
            Ok(MessageNode {
                parent_id: msg.parent_message_id,
                message: convert_db_message_to_base_message(msg.clone())?,
                children,
                sibling_index: idx as i32,
                depth,
            })
        })
        .collect()
}
