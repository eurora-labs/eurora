-- Migration: Stripe Billing Schema
-- Created: 2026-02-11 10:01:36
-- Description: Creates Stripe billing tables (customers, products, prices, subscriptions),
--              application-level plans/accounts, and an account billing state view.

----------------------------------------------------------------
-- Create dedicated schema for Stripe data
----------------------------------------------------------------
CREATE SCHEMA IF NOT EXISTS stripe;

----------------------------------------------------------------
-- Create stripe.customers table
-- Links Stripe customer records to application users
----------------------------------------------------------------
CREATE TABLE stripe.customers (
    id TEXT PRIMARY KEY,                          -- Stripe customer ID: cus_xxx
    app_user_id UUID,                             -- Application user reference
    email TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL,                      -- Full Stripe customer object

    CONSTRAINT fk_stripe_customers_app_user_id
        FOREIGN KEY (app_user_id)
        REFERENCES users(id)
        ON DELETE SET NULL
);

----------------------------------------------------------------
-- Create stripe.products table
-- High-level Stripe product offerings
----------------------------------------------------------------
CREATE TABLE stripe.products (
    id TEXT PRIMARY KEY,                          -- Stripe product ID: prod_xxx
    name TEXT NOT NULL,
    description TEXT,
    active BOOLEAN NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL
);

----------------------------------------------------------------
-- Create stripe.prices table
-- Concrete billing tiers and cadences for products
----------------------------------------------------------------
CREATE TABLE stripe.prices (
    id TEXT PRIMARY KEY,                          -- Stripe price ID: price_xxx
    product_id TEXT NOT NULL,
    nickname TEXT,
    currency TEXT NOT NULL,
    unit_amount BIGINT,                           -- In smallest currency unit (e.g. cents)
    recurring_interval TEXT,                       -- month / year / etc.
    recurring_interval_count INT,
    active BOOLEAN NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    raw_data JSONB NOT NULL,

    CONSTRAINT fk_stripe_prices_product_id
        FOREIGN KEY (product_id)
        REFERENCES stripe.products(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create ENUM type for Stripe subscription status
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Create stripe.subscriptions table
-- Core billing state: tracks what tier/plan a customer is on
----------------------------------------------------------------
CREATE TABLE stripe.subscriptions (
    id TEXT PRIMARY KEY,                          -- Stripe subscription ID: sub_xxx
    customer_id TEXT NOT NULL,
    status stripe.subscription_status NOT NULL,
    cancel_at_period_end BOOLEAN NOT NULL,
    canceled_at TIMESTAMP WITH TIME ZONE,
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    trial_start TIMESTAMP WITH TIME ZONE,
    trial_end TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    collection_method TEXT,                       -- charge_automatically / send_invoice
    default_payment_method TEXT,
    raw_data JSONB NOT NULL,

    CONSTRAINT fk_stripe_subscriptions_customer_id
        FOREIGN KEY (customer_id)
        REFERENCES stripe.customers(id)
        ON DELETE CASCADE
);

----------------------------------------------------------------
-- Create stripe.subscription_items table
-- One subscription may have multiple prices
----------------------------------------------------------------
CREATE TABLE stripe.subscription_items (
    id TEXT PRIMARY KEY,                          -- Stripe subscription_item ID: si_xxx
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

----------------------------------------------------------------
-- Create accounts table
-- Tenant / workspace in the application, linked to users
----------------------------------------------------------------
CREATE TABLE accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_user_id UUID NOT NULL,
    name TEXT NOT NULL,
    stripe_customer_id TEXT UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),

    CONSTRAINT fk_accounts_owner_user_id
        FOREIGN KEY (owner_user_id)
        REFERENCES users(id)
        ON DELETE CASCADE,

    CONSTRAINT fk_accounts_stripe_customer_id
        FOREIGN KEY (stripe_customer_id)
        REFERENCES stripe.customers(id)
        ON DELETE SET NULL
);

----------------------------------------------------------------
-- Create plans table
-- Logical plan/tier definitions for the application
----------------------------------------------------------------
CREATE TABLE plans (
    id TEXT PRIMARY KEY,                          -- e.g. 'free', 'pro', 'enterprise'
    name TEXT NOT NULL,
    description TEXT,
    max_users INT,
    max_projects INT,
    extra_metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

----------------------------------------------------------------
-- Create plan_prices junction table
-- Maps application plans to one or more Stripe prices
----------------------------------------------------------------
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

----------------------------------------------------------------
-- Create indexes for performance
----------------------------------------------------------------

-- Stripe customers indexes
CREATE INDEX idx_stripe_customers_app_user_id ON stripe.customers(app_user_id);
CREATE INDEX idx_stripe_customers_email ON stripe.customers(email);

-- Stripe prices indexes
CREATE INDEX idx_stripe_prices_product_id ON stripe.prices(product_id);
CREATE INDEX idx_stripe_prices_active ON stripe.prices(active);
CREATE INDEX idx_stripe_prices_recurring_interval ON stripe.prices(recurring_interval);

-- Stripe subscriptions indexes
CREATE INDEX idx_stripe_subscriptions_customer_id_created_at ON stripe.subscriptions(customer_id, created_at DESC);
CREATE INDEX idx_stripe_subscriptions_status ON stripe.subscriptions(status);
CREATE INDEX idx_stripe_subscriptions_current_period_end ON stripe.subscriptions(current_period_end);

-- Stripe subscription items indexes
CREATE INDEX idx_stripe_subscription_items_subscription_id ON stripe.subscription_items(subscription_id);
CREATE INDEX idx_stripe_subscription_items_price_id ON stripe.subscription_items(price_id);

-- Accounts indexes
CREATE INDEX idx_accounts_owner_user_id ON accounts(owner_user_id);

-- Plan prices indexes (PK covers plan_id already)
CREATE INDEX idx_plan_prices_stripe_price_id ON plan_prices(stripe_price_id);

----------------------------------------------------------------
-- Add triggers for automatic updated_at timestamp updates
-- Uses existing update_updated_at_column() function from initial migration
----------------------------------------------------------------
CREATE TRIGGER update_stripe_customers_updated_at
    BEFORE UPDATE ON stripe.customers
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_stripe_products_updated_at
    BEFORE UPDATE ON stripe.products
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

CREATE TRIGGER update_accounts_updated_at
    BEFORE UPDATE ON accounts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_plans_updated_at
    BEFORE UPDATE ON plans
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Create account billing state view
-- Provides a convenient summary of each account's current billing state
----------------------------------------------------------------
CREATE VIEW account_billing_state AS
SELECT DISTINCT ON (a.id)
    a.id AS account_id,
    s.id AS stripe_subscription_id,
    s.status,
    s.current_period_start,
    s.current_period_end,
    s.cancel_at_period_end,
    p.id AS plan_id,
    p.name AS plan_name,
    p.max_users,
    p.max_projects
FROM accounts a
LEFT JOIN stripe.customers c
    ON c.id = a.stripe_customer_id
LEFT JOIN LATERAL (
    SELECT
        sub.id,
        sub.customer_id,
        sub.status,
        sub.current_period_start,
        sub.current_period_end,
        sub.cancel_at_period_end,
        sub.created_at
    FROM stripe.subscriptions sub
    WHERE sub.customer_id = c.id
    ORDER BY sub.created_at DESC
    LIMIT 1
) s ON TRUE
LEFT JOIN stripe.subscription_items si
    ON si.subscription_id = s.id
LEFT JOIN stripe.prices sp
    ON sp.id = si.price_id
LEFT JOIN plan_prices pp
    ON pp.stripe_price_id = sp.id
LEFT JOIN plans p
    ON p.id = pp.plan_id
ORDER BY a.id;

----------------------------------------------------------------
-- Add comments for documentation
----------------------------------------------------------------

-- Stripe schema
COMMENT ON SCHEMA stripe IS 'Stripe billing data mirrored from webhook events';

-- Stripe customers
COMMENT ON TABLE stripe.customers IS 'Stripe customer records linked to application users';
COMMENT ON COLUMN stripe.customers.id IS 'Stripe customer ID (cus_xxx)';
COMMENT ON COLUMN stripe.customers.app_user_id IS 'Foreign key to users table';
COMMENT ON COLUMN stripe.customers.email IS 'Customer email from Stripe';
COMMENT ON COLUMN stripe.customers.updated_at IS 'Last time this record was updated locally';
COMMENT ON COLUMN stripe.customers.raw_data IS 'Full Stripe customer JSON object';

-- Stripe products
COMMENT ON TABLE stripe.products IS 'Stripe product definitions';
COMMENT ON COLUMN stripe.products.id IS 'Stripe product ID (prod_xxx)';
COMMENT ON COLUMN stripe.products.name IS 'Product display name';
COMMENT ON COLUMN stripe.products.active IS 'Whether the product is currently available';
COMMENT ON COLUMN stripe.products.metadata IS 'Stripe product metadata key-value pairs';
COMMENT ON COLUMN stripe.products.updated_at IS 'Last time this record was updated locally';
COMMENT ON COLUMN stripe.products.raw_data IS 'Full Stripe product JSON object';

-- Stripe prices
COMMENT ON TABLE stripe.prices IS 'Stripe price definitions linked to products';
COMMENT ON COLUMN stripe.prices.id IS 'Stripe price ID (price_xxx)';
COMMENT ON COLUMN stripe.prices.product_id IS 'Foreign key to stripe.products';
COMMENT ON COLUMN stripe.prices.unit_amount IS 'Price in smallest currency unit (e.g. cents)';
COMMENT ON COLUMN stripe.prices.recurring_interval IS 'Billing interval: month, year, etc.';
COMMENT ON COLUMN stripe.prices.updated_at IS 'Last time this record was updated locally';
COMMENT ON COLUMN stripe.prices.raw_data IS 'Full Stripe price JSON object';

-- Stripe subscriptions
COMMENT ON TABLE stripe.subscriptions IS 'Stripe subscription records tracking customer billing state';
COMMENT ON COLUMN stripe.subscriptions.id IS 'Stripe subscription ID (sub_xxx)';
COMMENT ON COLUMN stripe.subscriptions.customer_id IS 'Foreign key to stripe.customers';
COMMENT ON COLUMN stripe.subscriptions.status IS 'Current subscription status';
COMMENT ON COLUMN stripe.subscriptions.cancel_at_period_end IS 'Whether subscription cancels at period end';
COMMENT ON COLUMN stripe.subscriptions.current_period_start IS 'Start of current billing period';
COMMENT ON COLUMN stripe.subscriptions.current_period_end IS 'End of current billing period';
COMMENT ON COLUMN stripe.subscriptions.updated_at IS 'Last time this record was updated locally';
COMMENT ON COLUMN stripe.subscriptions.raw_data IS 'Full Stripe subscription JSON object';

-- Stripe subscription items
COMMENT ON TABLE stripe.subscription_items IS 'Individual line items within a subscription';
COMMENT ON COLUMN stripe.subscription_items.id IS 'Stripe subscription item ID (si_xxx)';
COMMENT ON COLUMN stripe.subscription_items.subscription_id IS 'Foreign key to stripe.subscriptions';
COMMENT ON COLUMN stripe.subscription_items.price_id IS 'Foreign key to stripe.prices';
COMMENT ON COLUMN stripe.subscription_items.quantity IS 'Quantity of this price in the subscription';
COMMENT ON COLUMN stripe.subscription_items.raw_data IS 'Full Stripe subscription item JSON object';

-- Accounts
COMMENT ON TABLE accounts IS 'Application tenant/workspace accounts linked to users and Stripe customers';
COMMENT ON COLUMN accounts.id IS 'Primary key UUID for account';
COMMENT ON COLUMN accounts.owner_user_id IS 'Foreign key to users table - account owner';
COMMENT ON COLUMN accounts.name IS 'Display name for the account/workspace';
COMMENT ON COLUMN accounts.stripe_customer_id IS 'Foreign key to stripe.customers (unique per account)';

-- Plans
COMMENT ON TABLE plans IS 'Application plan/tier definitions with feature limits';
COMMENT ON COLUMN plans.id IS 'Plan identifier (e.g. free, pro, enterprise)';
COMMENT ON COLUMN plans.name IS 'Display name for the plan';
COMMENT ON COLUMN plans.max_users IS 'Maximum number of users allowed on this plan';
COMMENT ON COLUMN plans.max_projects IS 'Maximum number of projects allowed on this plan';
COMMENT ON COLUMN plans.extra_metadata IS 'Additional plan configuration as JSON';

-- Plan prices
COMMENT ON TABLE plan_prices IS 'Maps application plans to Stripe prices (many-to-many)';
COMMENT ON COLUMN plan_prices.plan_id IS 'Foreign key to plans table';
COMMENT ON COLUMN plan_prices.stripe_price_id IS 'Foreign key to stripe.prices';

-- Billing state view
COMMENT ON VIEW account_billing_state IS 'Summarizes each account''s current subscription and plan state';

-- Enum type
COMMENT ON TYPE stripe.subscription_status IS 'Stripe subscription lifecycle states';
