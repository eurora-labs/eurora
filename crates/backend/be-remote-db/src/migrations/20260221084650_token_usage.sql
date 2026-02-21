CREATE TABLE token_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    thread_id UUID NOT NULL,
    message_id UUID NOT NULL,
    input_tokens BIGINT NOT NULL,
    output_tokens BIGINT NOT NULL,
    reasoning_tokens BIGINT NOT NULL DEFAULT 0,
    cache_creation_tokens BIGINT NOT NULL DEFAULT 0,
    cache_read_tokens BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT fk_token_usage_user_id
        FOREIGN KEY (user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_token_usage_thread_id
        FOREIGN KEY (thread_id)
        REFERENCES threads(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_token_usage_message_id
        FOREIGN KEY (message_id)
        REFERENCES messages(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_token_usage_user_month ON token_usage (user_id, created_at);

CREATE TABLE monthly_token_totals (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    year_month INT NOT NULL,
    total_tokens BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (user_id, year_month)
);

ALTER TABLE plans ADD COLUMN monthly_token_limit BIGINT NOT NULL DEFAULT 0;

UPDATE plans SET monthly_token_limit = 0 WHERE id = 'free';
UPDATE plans SET monthly_token_limit = 2000000 WHERE id = 'tier1';
