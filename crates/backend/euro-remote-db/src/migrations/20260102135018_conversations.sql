-- Migration: Conversations and Messages Schema
-- Created: 2026-01-02 12:22:00
-- Description: Creates tables for storing LLM conversations, messages, activities, and assets
--              Designed to work seamlessly with agent-chain message types

----------------------------------------------------------------
-- Enable pg_uuidv7 extension for UUID v7 support
-- UUID v7 is time-ordered which is better for database performance (clustering)
-- Install on server: See https://github.com/fboulnois/pg_uuidv7
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "pg_uuidv7";

----------------------------------------------------------------
-- Create ENUM type for message types
-- Matches agent-chain-core's BaseMessage variants that are stored
----------------------------------------------------------------
CREATE TYPE message_type AS ENUM ('human', 'system', 'ai', 'tool');

----------------------------------------------------------------
-- Create conversations table
-- Stores chat conversation metadata linked to users
----------------------------------------------------------------
CREATE TABLE conversations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    user_id UUID NOT NULL,
    title VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    -- Foreign key to users table
    CONSTRAINT fk_conversations_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create messages table
-- Stores agent-chain BaseMessage types in a normalized form
----------------------------------------------------------------
CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    conversation_id UUID NOT NULL,
    message_type message_type NOT NULL,

    -- Content stored as JSONB for efficient querying
    -- For human: MessageContent (can be {"Text": "..."} or {"Parts": [...]})
    -- For system/ai/tool: Simple string stored as JSON string
    content JSONB NOT NULL,

    -- For ToolMessage: the ID of the tool call this responds to
    tool_call_id VARCHAR(255),

    -- For AIMessage: JSON array of ToolCall objects
    -- Each ToolCall: {"id": "...", "name": "...", "args": {...}}
    tool_calls JSONB,

    -- Additional metadata as JSON object
    additional_kwargs JSONB NOT NULL DEFAULT '{}',

    -- Ordering within conversation (messages sorted by this)
    sequence_num INTEGER NOT NULL,

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    -- Foreign key constraint
    CONSTRAINT fk_messages_conversation_id
        FOREIGN KEY (conversation_id)
        REFERENCES conversations(id)
        ON DELETE CASCADE,

    -- Ensure unique sequence number per conversation
    CONSTRAINT uq_messages_conversation_sequence
        UNIQUE (conversation_id, sequence_num),

    -- ToolMessage must have tool_call_id
    CONSTRAINT ck_messages_tool_call_id
        CHECK (message_type != 'tool' OR tool_call_id IS NOT NULL)
);

----------------------------------------------------------------
-- Create assets table (MUST be created before activities)
-- Stores file metadata (screenshots, attachments, etc.)
-- Actual file content stored externally (e.g., S3, filesystem)
----------------------------------------------------------------
CREATE TABLE assets (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    user_id UUID NOT NULL,

    name TEXT NOT NULL,
    mime_type TEXT NOT NULL, -- MIME type of the asset
    size_bytes BIGINT CHECK (size_bytes IS NULL OR size_bytes >= 0),
    checksum_sha256 bytea,

    storage_backend TEXT NOT NULL, -- Backend used for storage (e.g., S3, filesystem)
    storage_uri TEXT NOT NULL, -- Path to the file, could be s3 path or local server

    status TEXT NOT NULL DEFAULT 'uploaded',

    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    -- Additional metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Foreign key constraints
    CONSTRAINT fk_assets_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    -- SHA256 length check
    CONSTRAINT assets_sha256_len CHECK (checksum_sha256 IS NULL OR octet_length(checksum_sha256) = 32)
);

----------------------------------------------------------------
-- Create activities table
-- Tracks application/process usage (e.g., desktop apps, browser tabs)
----------------------------------------------------------------
CREATE TABLE activities (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v7(),
    user_id UUID NOT NULL,
    name VARCHAR(500) NOT NULL,
    icon_asset_id UUID NOT NULL,
    process_name VARCHAR(500) NOT NULL,
    window_title VARCHAR(500) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ended_at TIMESTAMP WITH TIME ZONE,

    -- Foreign key to users table
    CONSTRAINT fk_activities_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    -- Foreign key to assets table
    CONSTRAINT fk_activities_icon_asset_id
        FOREIGN KEY (icon_asset_id)
        REFERENCES assets(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create activity_conversations junction table
-- Links activities to conversations (many-to-many)
----------------------------------------------------------------
CREATE TABLE activity_conversations (
    activity_id UUID NOT NULL,
    conversation_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    PRIMARY KEY (activity_id, conversation_id),

    CONSTRAINT fk_activity_conversations_activity_id
        FOREIGN KEY (activity_id)
        REFERENCES activities(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activity_conversations_conversation_id
        FOREIGN KEY (conversation_id)
        REFERENCES conversations(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create activity_assets junction table
-- Links activities to assets (many-to-many)
----------------------------------------------------------------
CREATE TABLE activity_assets (
    activity_id UUID NOT NULL,
    asset_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    PRIMARY KEY (activity_id, asset_id),

    CONSTRAINT fk_activity_assets_activity_id
        FOREIGN KEY (activity_id)
        REFERENCES activities(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activity_assets_asset_id
        FOREIGN KEY (asset_id)
        REFERENCES assets(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create message_assets junction table
-- Links messages to assets (e.g., images attached to messages)
----------------------------------------------------------------
CREATE TABLE message_assets (
    message_id UUID NOT NULL,
    asset_id UUID NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    PRIMARY KEY (message_id, asset_id),

    CONSTRAINT fk_message_assets_message_id
        FOREIGN KEY (message_id)
        REFERENCES messages(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_message_assets_asset_id
        FOREIGN KEY (asset_id)
        REFERENCES assets(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------

-- Conversations indexes
CREATE INDEX idx_conversations_user_id ON conversations(user_id);
CREATE INDEX idx_conversations_updated_at ON conversations(updated_at DESC);
CREATE INDEX idx_conversations_user_updated ON conversations(user_id, updated_at DESC);

-- Messages indexes
CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
-- Note: uq_messages_conversation_sequence unique constraint already indexes (conversation_id, sequence_num)
CREATE INDEX idx_messages_type ON messages(message_type);
-- GIN index for JSONB content queries (if needed for full-text search later)
CREATE INDEX idx_messages_content ON messages USING GIN (content jsonb_path_ops);

-- Activities indexes
CREATE INDEX idx_activities_user_id ON activities(user_id);
CREATE INDEX idx_activities_started_at ON activities(started_at DESC);
CREATE INDEX idx_activities_user_started ON activities(user_id, started_at DESC);
CREATE INDEX idx_activities_ended_at ON activities(ended_at) WHERE ended_at IS NULL;

-- Activity conversations indexes
-- Note: Primary key (activity_id, conversation_id) already indexes activity_id
CREATE INDEX idx_activity_conversations_conversation_id ON activity_conversations(conversation_id);

-- Activity assets indexes
-- Note: Primary key (activity_id, asset_id) already indexes activity_id
CREATE INDEX idx_activity_assets_asset_id ON activity_assets(asset_id);

-- Assets indexes
CREATE INDEX idx_assets_user_id ON assets(user_id);
CREATE INDEX idx_assets_storage_uri ON assets(storage_uri);

-- Message assets indexes
-- Note: Primary key (message_id, asset_id) already indexes message_id
CREATE INDEX idx_message_assets_asset_id ON message_assets(asset_id);

----------------------------------------------------------------
-- Add triggers for automatic updated_at timestamp updates
-- Uses existing update_updated_at_column() function from initial migration
----------------------------------------------------------------
CREATE TRIGGER update_conversations_updated_at
    BEFORE UPDATE ON conversations
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_messages_updated_at
    BEFORE UPDATE ON messages
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_assets_updated_at
    BEFORE UPDATE ON assets
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Add comments for documentation
----------------------------------------------------------------

-- Conversations table
COMMENT ON TABLE conversations IS 'Chat conversations linked to user accounts';
COMMENT ON COLUMN conversations.id IS 'Primary key UUID for conversation';
COMMENT ON COLUMN conversations.user_id IS 'Foreign key to users table - owner of the conversation';
COMMENT ON COLUMN conversations.title IS 'Optional title for the conversation';
COMMENT ON COLUMN conversations.created_at IS 'Timestamp when conversation was created';
COMMENT ON COLUMN conversations.updated_at IS 'Timestamp when conversation was last updated';

-- Messages table
COMMENT ON TABLE messages IS 'Messages within conversations - stores agent-chain BaseMessage types';
COMMENT ON COLUMN messages.id IS 'Primary key UUID for message';
COMMENT ON COLUMN messages.conversation_id IS 'Foreign key to conversations table';
COMMENT ON COLUMN messages.message_type IS 'Type of message: human, system, ai, or tool';
COMMENT ON COLUMN messages.content IS 'Message content as JSONB - structure depends on message_type';
COMMENT ON COLUMN messages.tool_call_id IS 'For tool messages: ID of the tool call this responds to';
COMMENT ON COLUMN messages.tool_calls IS 'For AI messages: array of tool call requests';
COMMENT ON COLUMN messages.additional_kwargs IS 'Additional metadata for the message';
COMMENT ON COLUMN messages.sequence_num IS 'Order of message within conversation';

-- Activities table
COMMENT ON TABLE activities IS 'Tracked user activities (applications, browser tabs, etc.)';
COMMENT ON COLUMN activities.id IS 'Primary key UUID for activity';
COMMENT ON COLUMN activities.user_id IS 'Foreign key to users table';
COMMENT ON COLUMN activities.name IS 'Display name of the activity';
COMMENT ON COLUMN activities.icon_asset_id IS 'Foreign key to assets table for activity icon';
COMMENT ON COLUMN activities.process_name IS 'System process name';
COMMENT ON COLUMN activities.started_at IS 'When the activity started';
COMMENT ON COLUMN activities.ended_at IS 'When the activity ended (NULL if still active)';

-- Activity conversations table
COMMENT ON TABLE activity_conversations IS 'Links activities to related conversations';
COMMENT ON COLUMN activity_conversations.activity_id IS 'Foreign key to activities table';
COMMENT ON COLUMN activity_conversations.conversation_id IS 'Foreign key to conversations table';

-- Activity assets table
COMMENT ON TABLE activity_assets IS 'Links activities to their associated assets (many-to-many)';
COMMENT ON COLUMN activity_assets.activity_id IS 'Foreign key to activities table';
COMMENT ON COLUMN activity_assets.asset_id IS 'Foreign key to assets table';

-- Assets table
COMMENT ON TABLE assets IS 'File assets (screenshots, attachments) with external storage';
COMMENT ON COLUMN assets.id IS 'Primary key UUID for asset';
COMMENT ON COLUMN assets.user_id IS 'Foreign key to users table - owner of the asset';
COMMENT ON COLUMN assets.checksum_sha256 IS 'SHA256 hash of the file content for deduplication';
COMMENT ON COLUMN assets.size_bytes IS 'Size of the asset in bytes';
COMMENT ON COLUMN assets.storage_backend IS 'Storage backend type (e.g., S3, local)';
COMMENT ON COLUMN assets.storage_uri IS 'Path to the file (S3 key, local path, etc.)';
COMMENT ON COLUMN assets.mime_type IS 'MIME type of the asset';
COMMENT ON COLUMN assets.metadata IS 'Additional metadata as JSON';

-- Message assets table
COMMENT ON TABLE message_assets IS 'Links messages to their attached assets';
COMMENT ON COLUMN message_assets.message_id IS 'Foreign key to messages table';
COMMENT ON COLUMN message_assets.asset_id IS 'Foreign key to assets table';

-- Enum type
COMMENT ON TYPE message_type IS 'Enum for message types matching agent-chain BaseMessage variants';
