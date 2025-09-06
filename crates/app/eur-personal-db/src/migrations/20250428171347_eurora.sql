-- Create tables for Eurora database

-- Table for conversation history
CREATE TABLE conversation (
    id TEXT PRIMARY KEY,         -- UUID
    title TEXT NOT NULL,          -- Conversation name
    created_at TEXT NOT NULL,    -- ISO8601 datetime when conversation was created
    updated_at TEXT NOT NULL    -- ISO8601 datetime when conversation was last updated
);

-- Table for conversation messages
CREATE TABLE chat_message (
    id TEXT PRIMARY KEY,         -- UUID
    conversation_id TEXT NOT NULL, -- Foreign key to conversation

    role TEXT NOT NULL,          -- Role of the message sender (user or assistant)
    content TEXT NOT NULL,       -- Content of the message

    has_assets BOOLEAN NOT NULL DEFAULT 0, -- Whether the message has assets (true or false)

    created_at TEXT NOT NULL,    -- ISO8601 datetime when message was created
    updated_at TEXT NOT NULL,    -- ISO8601 datetime when message was last updated
    FOREIGN KEY (conversation_id) REFERENCES conversation(id) ON DELETE CASCADE
);


-- Table for tracking each individual activity
CREATE TABLE activity (
    id TEXT PRIMARY KEY,         -- UUID
    name TEXT NOT NULL,          -- Activity name
    icon_path TEXT,     -- Path to the icon
    process_name TEXT NOT NULL,  -- Process name
    started_at TEXT NOT NULL,    -- ISO8601 datetime when activity started
    ended_at TEXT                -- ISO8601 datetime when activity ended (nullable)
);

-- Table for activity to conversation mapping
CREATE TABLE activity_conversation (
    activity_id TEXT NOT NULL,   -- Foreign key to activity
    conversation_id TEXT NOT NULL, -- Foreign key to conversation
    created_at TEXT NOT NULL,    -- ISO8601 datetime when mapping was created
    PRIMARY KEY (activity_id, conversation_id),
    FOREIGN KEY (activity_id) REFERENCES activity(id) ON DELETE CASCADE,
    FOREIGN KEY (conversation_id) REFERENCES conversation(id) ON DELETE CASCADE
);

-- Table for references to heavier prompt helpers
CREATE TABLE asset (
    id TEXT PRIMARY KEY,         -- UUID
    activity_id TEXT,   -- Foreign key to activity
    relative_path TEXT NOT NULL,
    absolute_path TEXT NOT NULL,
    created_at TEXT NOT NULL,    -- ISO8601 datetime when asset was created
    updated_at TEXT NOT NULL,    -- ISO8601 datetime when asset was last updated

    FOREIGN KEY (activity_id) REFERENCES activity(id) ON DELETE SET NULL
);

-- Table for chat_message to asset mapping
CREATE TABLE chat_message_asset (
    chat_message_id TEXT NOT NULL,   -- Foreign key to chat_message
    asset_id TEXT NOT NULL, -- Foreign key to asset
    created_at TEXT NOT NULL,    -- ISO8601 datetime when mapping was created
    PRIMARY KEY (chat_message_id, asset_id),
    FOREIGN KEY (chat_message_id) REFERENCES chat_message(id) ON DELETE CASCADE,
    FOREIGN KEY (asset_id) REFERENCES asset(id) ON DELETE CASCADE

);

-- Create indexes for foreign keys to improve query performance
CREATE INDEX idx_asset_activity_id ON asset(activity_id);
CREATE INDEX idx_chat_message_conversation_id ON chat_message(conversation_id);
CREATE INDEX idx_chat_message_asset_asset_id ON chat_message_asset(asset_id);

CREATE INDEX idx_activity_conversation_activity_id ON activity_conversation(activity_id);
CREATE INDEX idx_activity_conversation_conversation_id ON activity_conversation(conversation_id);
