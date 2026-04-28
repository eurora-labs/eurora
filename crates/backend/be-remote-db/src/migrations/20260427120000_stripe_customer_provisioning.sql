-- Outbox for eager Stripe customer provisioning.
--
-- A row is inserted in the same transaction as user creation. A background
-- worker drains the outbox by calling Stripe and writing back into
-- stripe.customers + users.stripe_customer_id. The row is deleted in the
-- same transaction as the link write. Permanent failures (4xx from Stripe)
-- flip dead_letter so the drainer stops retrying.
CREATE TABLE stripe.customer_provisioning_jobs (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    last_error TEXT,
    dead_letter BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE INDEX idx_customer_provisioning_jobs_due
    ON stripe.customer_provisioning_jobs(next_attempt_at)
    WHERE dead_letter = FALSE;

CREATE TRIGGER update_customer_provisioning_jobs_updated_at
    BEFORE UPDATE ON stripe.customer_provisioning_jobs
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Backfill: anyone currently without a Stripe customer needs one.
-- The drainer will pick these up on the next deploy.
INSERT INTO stripe.customer_provisioning_jobs (user_id)
SELECT id FROM users WHERE stripe_customer_id IS NULL
ON CONFLICT (user_id) DO NOTHING;
