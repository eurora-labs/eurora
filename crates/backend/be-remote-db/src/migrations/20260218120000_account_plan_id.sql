INSERT INTO plans (id, name, description)
VALUES
    ('free',  'Free', 'Free tier'),
    ('tier1', 'Pro',  'Pro tier')
ON CONFLICT (id) DO NOTHING;

ALTER TABLE users
    ADD COLUMN plan_id TEXT NOT NULL DEFAULT 'free'
    REFERENCES plans(id);

CREATE INDEX idx_users_plan_id ON users(plan_id);
