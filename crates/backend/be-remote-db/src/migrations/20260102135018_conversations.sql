CREATE TYPE message_type AS ENUM ('human', 'system', 'ai', 'tool');

CREATE TABLE conversations (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    title VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_conversations_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE
);

CREATE TABLE messages (
    id UUID PRIMARY KEY,
    conversation_id UUID NOT NULL,
    user_id UUID NOT NULL,
    message_type message_type NOT NULL,
    content JSONB NOT NULL,
    tool_call_id VARCHAR(255),
    tool_calls JSONB,
    additional_kwargs JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_messages_conversation_id
        FOREIGN KEY (conversation_id)
        REFERENCES conversations(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_messages_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    -- ToolMessage must have tool_call_id
    CONSTRAINT messages_chk_tool_call_id
        CHECK (message_type != 'tool' OR tool_call_id IS NOT NULL)
);

CREATE TYPE asset_status AS ENUM ('pending', 'uploaded', 'processing', 'ready', 'failed', 'deleted');

CREATE TABLE assets (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT CHECK (size_bytes IS NULL OR size_bytes >= 0),
    checksum_sha256 bytea,
    storage_backend TEXT NOT NULL,
    storage_uri TEXT NOT NULL,
    status asset_status NOT NULL DEFAULT 'uploaded',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    metadata JSONB NOT NULL DEFAULT '{}',

    CONSTRAINT fk_assets_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT assets_chk_sha256_len CHECK (checksum_sha256 IS NULL OR octet_length(checksum_sha256) = 32)
);

CREATE TABLE activities (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    name VARCHAR(500) NOT NULL,
    icon_asset_id UUID,
    process_name VARCHAR(500) NOT NULL,
    window_title VARCHAR(500) NOT NULL,
    started_at TIMESTAMP WITH TIME ZONE NOT NULL,
    ended_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_activities_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_activities_icon_asset_id
        FOREIGN KEY (icon_asset_id)
        REFERENCES assets(id)
        ON DELETE SET NULL
);

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

CREATE INDEX idx_conversations_user_id ON conversations(user_id);
CREATE INDEX idx_conversations_updated_at ON conversations(updated_at DESC);
CREATE INDEX idx_conversations_user_updated ON conversations(user_id, updated_at DESC);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_type ON messages(message_type);
CREATE INDEX idx_messages_content ON messages USING GIN (content jsonb_path_ops);

CREATE INDEX idx_activities_user_id ON activities(user_id);
CREATE INDEX idx_activities_started_at ON activities(started_at DESC);
CREATE INDEX idx_activities_user_started ON activities(user_id, started_at DESC);
CREATE INDEX idx_activities_ended_at ON activities(ended_at) WHERE ended_at IS NULL;

CREATE INDEX idx_activity_conversations_conversation_id ON activity_conversations(conversation_id);
CREATE INDEX idx_activity_assets_asset_id ON activity_assets(asset_id);
CREATE INDEX idx_assets_user_id ON assets(user_id);
CREATE INDEX idx_assets_storage_uri ON assets(storage_uri);
CREATE INDEX idx_message_assets_asset_id ON message_assets(asset_id);

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

CREATE TRIGGER update_activities_updated_at
    BEFORE UPDATE ON activities
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
