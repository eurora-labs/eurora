CREATE SCHEMA IF NOT EXISTS stripe;

CREATE TABLE stripe.customers (
    id TEXT PRIMARY KEY,
    app_user_id UUID,
    email TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL,

    CONSTRAINT fk_stripe_customers_app_user_id
        FOREIGN KEY (app_user_id)
        REFERENCES users(id)
        ON DELETE SET NULL
);

CREATE TABLE stripe.prices (
    id TEXT PRIMARY KEY,
    currency TEXT NOT NULL,
    unit_amount BIGINT,
    recurring_interval TEXT,
    active BOOLEAN NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL
);

CREATE TYPE stripe.subscription_status AS ENUM (
    'incomplete',
    'incomplete_expired',
    'trialing',
    'active',
    'past_due',
    'canceled',
    'unpaid',
    'paused'
);

CREATE TABLE stripe.subscriptions (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL,
    status stripe.subscription_status NOT NULL,
    cancel_at_period_end BOOLEAN NOT NULL,
    canceled_at TIMESTAMP WITH TIME ZONE,
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL,

    CONSTRAINT fk_stripe_subscriptions_customer_id
        FOREIGN KEY (customer_id)
        REFERENCES stripe.customers(id)
        ON DELETE CASCADE
);

CREATE TABLE stripe.subscription_items (
    id TEXT PRIMARY KEY,
    subscription_id TEXT NOT NULL,
    price_id TEXT NOT NULL,
    quantity INTEGER,
    raw_data JSONB NOT NULL,

    CONSTRAINT fk_stripe_subscription_items_subscription_id
        FOREIGN KEY (subscription_id)
        REFERENCES stripe.subscriptions(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_stripe_subscription_items_price_id
        FOREIGN KEY (price_id)
        REFERENCES stripe.prices(id)
        ON DELETE CASCADE
);

ALTER TABLE users
    ADD COLUMN stripe_customer_id TEXT UNIQUE,
    ADD CONSTRAINT fk_users_stripe_customer_id
        FOREIGN KEY (stripe_customer_id)
        REFERENCES stripe.customers(id)
        ON DELETE SET NULL;

CREATE TABLE plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

CREATE TABLE plan_prices (
    plan_id TEXT NOT NULL,
    stripe_price_id TEXT NOT NULL,

    PRIMARY KEY (plan_id, stripe_price_id),

    CONSTRAINT fk_plan_prices_plan_id
        FOREIGN KEY (plan_id)
        REFERENCES plans(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_plan_prices_stripe_price_id
        FOREIGN KEY (stripe_price_id)
        REFERENCES stripe.prices(id)
        ON DELETE CASCADE
);

CREATE INDEX idx_stripe_customers_app_user_id ON stripe.customers(app_user_id);
CREATE INDEX idx_stripe_customers_email ON stripe.customers(email);
CREATE INDEX idx_stripe_prices_active ON stripe.prices(active);
CREATE INDEX idx_stripe_prices_recurring_interval ON stripe.prices(recurring_interval);
CREATE INDEX idx_stripe_subscriptions_customer_id_created_at ON stripe.subscriptions(customer_id, created_at DESC);
CREATE INDEX idx_stripe_subscriptions_status ON stripe.subscriptions(status);
CREATE INDEX idx_stripe_subscriptions_current_period_end ON stripe.subscriptions(current_period_end);
CREATE INDEX idx_stripe_subscription_items_subscription_id ON stripe.subscription_items(subscription_id);
CREATE INDEX idx_stripe_subscription_items_price_id ON stripe.subscription_items(price_id);
CREATE INDEX idx_plan_prices_stripe_price_id ON plan_prices(stripe_price_id);

CREATE TRIGGER update_stripe_customers_updated_at
    BEFORE UPDATE ON stripe.customers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_stripe_prices_updated_at
    BEFORE UPDATE ON stripe.prices
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_stripe_subscriptions_updated_at
    BEFORE UPDATE ON stripe.subscriptions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_plans_updated_at
    BEFORE UPDATE ON plans
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Idempotency tracking for webhook processing
CREATE TABLE stripe.webhook_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);
