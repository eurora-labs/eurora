INSERT INTO plans (id, name, description)
VALUES
    ('free',  'Free', 'Free tier'),
    ('tier1', 'Pro',  'Pro tier')
ON CONFLICT (id) DO NOTHING;

-- Source of truth for what plan a user is on, decoupled from Stripe
ALTER TABLE accounts
    ADD COLUMN plan_id TEXT NOT NULL DEFAULT 'free'
    REFERENCES plans(id);

CREATE INDEX idx_accounts_plan_id ON accounts(plan_id);
