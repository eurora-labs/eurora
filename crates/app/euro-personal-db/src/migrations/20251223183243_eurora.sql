-- Eurora Personal Database Schema
-- Designed to work seamlessly with agent-chain message types

-- Table for conversations
CREATE TABLE conversation (
    id TEXT PRIMARY KEY,                    -- UUID
    title TEXT,                             -- Optional conversation title
    created_at TEXT NOT NULL,               -- ISO8601 datetime
    updated_at TEXT NOT NULL                -- ISO8601 datetime
);

-- Table for messages (stores agent-chain BaseMessage types)
-- Supports: Human, System, AI, Tool message types
CREATE TABLE message (
    id TEXT PRIMARY KEY,                    -- UUID (from BaseMessage.id())
    conversation_id TEXT NOT NULL,          -- Foreign key to conversation
    message_type TEXT NOT NULL,             -- 'human', 'system', 'ai', 'tool'
    
    -- Content field stores JSON:
    -- For human: MessageContent (can be {"Text": "..."} or {"Parts": [...]})
    -- For system/ai/tool: Simple string stored as JSON string
    content TEXT NOT NULL,
    
    -- For ToolMessage: the ID of the tool call this responds to
    tool_call_id TEXT,
    
    -- For AIMessage: JSON array of ToolCall objects
    -- Each ToolCall: {"id": "...", "name": "...", "args": {...}}
    tool_calls TEXT,
    
    -- Additional metadata as JSON object
    additional_kwargs TEXT NOT NULL DEFAULT '{}',
    
    -- Ordering within conversation (messages sorted by this)
    sequence_num INTEGER NOT NULL,
    
    created_at TEXT NOT NULL,               -- ISO8601 datetime
    updated_at TEXT NOT NULL,               -- ISO8601 datetime
    
    FOREIGN KEY (conversation_id) REFERENCES conversation(id) ON DELETE CASCADE,
    
    -- Ensure proper message type values
    CHECK (message_type IN ('human', 'system', 'ai', 'tool')),
    -- ToolMessage must have tool_call_id
    CHECK (message_type != 'tool' OR tool_call_id IS NOT NULL)
);

-- Table for tracking activities (application/process usage)
CREATE TABLE activity (
    id TEXT PRIMARY KEY,                    -- UUID
    name TEXT NOT NULL,                     -- Activity name
    icon_path TEXT,                         -- Path to the icon
    process_name TEXT NOT NULL,             -- Process name
    started_at TEXT NOT NULL,               -- ISO8601 datetime
    ended_at TEXT                           -- ISO8601 datetime (nullable if still active)
);

-- Table for activity to conversation mapping
CREATE TABLE activity_conversation (
    activity_id TEXT NOT NULL,              -- Foreign key to activity
    conversation_id TEXT NOT NULL,          -- Foreign key to conversation
    created_at TEXT NOT NULL,               -- ISO8601 datetime
    PRIMARY KEY (activity_id, conversation_id),
    FOREIGN KEY (activity_id) REFERENCES activity(id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_id) REFERENCES conversation(id) ON DELETE CASCADE
);

-- Table for file assets (screenshots, files, etc.)
CREATE TABLE asset (
    id TEXT PRIMARY KEY,                    -- UUID
    activity_id TEXT,                       -- Optional foreign key to activity
    relative_path TEXT NOT NULL,            -- Relative path within storage
    absolute_path TEXT NOT NULL,            -- Absolute filesystem path
    created_at TEXT NOT NULL,               -- ISO8601 datetime
    updated_at TEXT NOT NULL,               -- ISO8601 datetime
    
    FOREIGN KEY (activity_id) REFERENCES activity(id) ON DELETE SET NULL
);

-- Table for message to asset mapping (e.g., images attached to messages)
CREATE TABLE message_asset (
    message_id TEXT NOT NULL,               -- Foreign key to message
    asset_id TEXT NOT NULL,                 -- Foreign key to asset
    created_at TEXT NOT NULL,               -- ISO8601 datetime
    PRIMARY KEY (message_id, asset_id),
    FOREIGN KEY (message_id) REFERENCES message(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES asset(id) ON DELETE CASCADE
);

-- Indexes for common query patterns
CREATE INDEX idx_message_conversation_id ON message(conversation_id);
CREATE INDEX idx_message_conversation_sequence ON message(conversation_id, sequence_num);
CREATE INDEX idx_asset_activity_id ON asset(activity_id);
CREATE INDEX idx_message_asset_asset_id ON message_asset(asset_id);
CREATE INDEX idx_activity_conversation_activity_id ON activity_conversation(activity_id);
CREATE INDEX idx_activity_conversation_conversation_id ON activity_conversation(conversation_id);
CREATE INDEX idx_conversation_updated_at ON conversation(updated_at);
CREATE INDEX idx_activity_started_at ON activity(started_at);