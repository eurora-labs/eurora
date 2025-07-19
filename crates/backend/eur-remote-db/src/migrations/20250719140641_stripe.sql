-- Migration: Stripe Integration
-- Created: 2025-07-19 14:06:41
-- Description: Adds comprehensive Stripe integration with products, subscriptions, payments, and webhooks

----------------------------------------------------------------
-- Enable UUID extension if not already enabled
----------------------------------------------------------------
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

----------------------------------------------------------------
-- Add stripe_customer_id to existing users table
----------------------------------------------------------------
ALTER TABLE users 
ADD COLUMN stripe_customer_id VARCHAR(255) UNIQUE;

-- Index for stripe_customer_id lookups
CREATE INDEX idx_users_stripe_customer_id ON users(stripe_customer_id);

----------------------------------------------------------------
-- Create products table
----------------------------------------------------------------
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_product_id VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    active BOOLEAN DEFAULT true,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now()
);

-- Indexes for products
CREATE INDEX idx_products_stripe_product_id ON products(stripe_product_id);
CREATE INDEX idx_products_active ON products(active);

----------------------------------------------------------------
-- Create prices table
----------------------------------------------------------------
CREATE TABLE prices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_price_id VARCHAR(255) UNIQUE NOT NULL,
    product_id UUID NOT NULL,
    active BOOLEAN DEFAULT true,
    currency VARCHAR(3) NOT NULL,
    unit_amount BIGINT, -- Amount in smallest currency unit (cents)
    recurring_interval VARCHAR(20), -- 'day', 'week', 'month', 'year'
    recurring_interval_count INTEGER DEFAULT 1,
    billing_scheme VARCHAR(20) DEFAULT 'per_unit', -- 'per_unit' or 'tiered'
    tiers JSONB, -- For tiered pricing
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraint
    CONSTRAINT fk_prices_product_id 
        FOREIGN KEY (product_id) 
        REFERENCES products(id) 
        ON DELETE CASCADE,
        
    -- Check constraints
    CONSTRAINT ck_prices_currency CHECK (LENGTH(currency) = 3),
    CONSTRAINT ck_prices_recurring_interval CHECK (
        recurring_interval IS NULL OR 
        recurring_interval IN ('day', 'week', 'month', 'year')
    ),
    CONSTRAINT ck_prices_billing_scheme CHECK (
        billing_scheme IN ('per_unit', 'tiered')
    )
);

-- Indexes for prices
CREATE INDEX idx_prices_stripe_price_id ON prices(stripe_price_id);
CREATE INDEX idx_prices_product_id ON prices(product_id);
CREATE INDEX idx_prices_active ON prices(active);
CREATE INDEX idx_prices_currency ON prices(currency);

----------------------------------------------------------------
-- Create subscriptions table
----------------------------------------------------------------
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_subscription_id VARCHAR(255) UNIQUE NOT NULL,
    user_id UUID NOT NULL,
    status VARCHAR(50) NOT NULL,
    current_period_start TIMESTAMP WITH TIME ZONE NOT NULL,
    current_period_end TIMESTAMP WITH TIME ZONE NOT NULL,
    cancel_at_period_end BOOLEAN DEFAULT false,
    canceled_at TIMESTAMP WITH TIME ZONE,
    ended_at TIMESTAMP WITH TIME ZONE,
    trial_start TIMESTAMP WITH TIME ZONE,
    trial_end TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraint
    CONSTRAINT fk_subscriptions_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE CASCADE,
        
    -- Check constraints
    CONSTRAINT ck_subscriptions_status CHECK (
        status IN ('incomplete', 'incomplete_expired', 'trialing', 'active', 
                  'past_due', 'canceled', 'unpaid', 'paused')
    )
);

-- Indexes for subscriptions
CREATE INDEX idx_subscriptions_stripe_subscription_id ON subscriptions(stripe_subscription_id);
CREATE INDEX idx_subscriptions_user_id ON subscriptions(user_id);
CREATE INDEX idx_subscriptions_status ON subscriptions(status);
CREATE INDEX idx_subscriptions_current_period_end ON subscriptions(current_period_end);

----------------------------------------------------------------
-- Create subscription_items table
----------------------------------------------------------------
CREATE TABLE subscription_items (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_subscription_item_id VARCHAR(255) UNIQUE NOT NULL,
    subscription_id UUID NOT NULL,
    price_id UUID NOT NULL,
    quantity INTEGER DEFAULT 1,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraints
    CONSTRAINT fk_subscription_items_subscription_id 
        FOREIGN KEY (subscription_id) 
        REFERENCES subscriptions(id) 
        ON DELETE CASCADE,
    CONSTRAINT fk_subscription_items_price_id 
        FOREIGN KEY (price_id) 
        REFERENCES prices(id) 
        ON DELETE RESTRICT,
        
    -- Check constraints
    CONSTRAINT ck_subscription_items_quantity CHECK (quantity > 0)
);

-- Indexes for subscription_items
CREATE INDEX idx_subscription_items_stripe_subscription_item_id ON subscription_items(stripe_subscription_item_id);
CREATE INDEX idx_subscription_items_subscription_id ON subscription_items(subscription_id);
CREATE INDEX idx_subscription_items_price_id ON subscription_items(price_id);

----------------------------------------------------------------
-- Create invoices table
----------------------------------------------------------------
CREATE TABLE invoices (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_invoice_id VARCHAR(255) UNIQUE NOT NULL,
    user_id UUID NOT NULL,
    subscription_id UUID,
    status VARCHAR(50) NOT NULL,
    currency VARCHAR(3) NOT NULL,
    amount_due BIGINT NOT NULL,
    amount_paid BIGINT DEFAULT 0,
    amount_remaining BIGINT DEFAULT 0,
    subtotal BIGINT NOT NULL,
    total BIGINT NOT NULL,
    tax BIGINT DEFAULT 0,
    hosted_invoice_url TEXT,
    invoice_pdf TEXT,
    period_start TIMESTAMP WITH TIME ZONE,
    period_end TIMESTAMP WITH TIME ZONE,
    due_date TIMESTAMP WITH TIME ZONE,
    paid_at TIMESTAMP WITH TIME ZONE,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraints
    CONSTRAINT fk_invoices_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE CASCADE,
    CONSTRAINT fk_invoices_subscription_id 
        FOREIGN KEY (subscription_id) 
        REFERENCES subscriptions(id) 
        ON DELETE SET NULL,
        
    -- Check constraints
    CONSTRAINT ck_invoices_status CHECK (
        status IN ('draft', 'open', 'paid', 'uncollectible', 'void')
    ),
    CONSTRAINT ck_invoices_currency CHECK (LENGTH(currency) = 3)
);

-- Indexes for invoices
CREATE INDEX idx_invoices_stripe_invoice_id ON invoices(stripe_invoice_id);
CREATE INDEX idx_invoices_user_id ON invoices(user_id);
CREATE INDEX idx_invoices_subscription_id ON invoices(subscription_id);
CREATE INDEX idx_invoices_status ON invoices(status);
CREATE INDEX idx_invoices_due_date ON invoices(due_date);

----------------------------------------------------------------
-- Create payments table
----------------------------------------------------------------
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_payment_intent_id VARCHAR(255) UNIQUE NOT NULL,
    user_id UUID NOT NULL,
    invoice_id UUID,
    amount BIGINT NOT NULL,
    currency VARCHAR(3) NOT NULL,
    status VARCHAR(50) NOT NULL,
    payment_method_type VARCHAR(50),
    payment_method_id VARCHAR(255),
    client_secret VARCHAR(255),
    confirmation_method VARCHAR(50),
    receipt_email VARCHAR(255),
    description TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraints
    CONSTRAINT fk_payments_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE CASCADE,
    CONSTRAINT fk_payments_invoice_id 
        FOREIGN KEY (invoice_id) 
        REFERENCES invoices(id) 
        ON DELETE SET NULL,
        
    -- Check constraints
    CONSTRAINT ck_payments_status CHECK (
        status IN ('requires_payment_method', 'requires_confirmation', 'requires_action',
                  'processing', 'requires_capture', 'canceled', 'succeeded')
    ),
    CONSTRAINT ck_payments_currency CHECK (LENGTH(currency) = 3),
    CONSTRAINT ck_payments_amount CHECK (amount > 0)
);

-- Indexes for payments
CREATE INDEX idx_payments_stripe_payment_intent_id ON payments(stripe_payment_intent_id);
CREATE INDEX idx_payments_user_id ON payments(user_id);
CREATE INDEX idx_payments_invoice_id ON payments(invoice_id);
CREATE INDEX idx_payments_status ON payments(status);

----------------------------------------------------------------
-- Create checkout_sessions table
----------------------------------------------------------------
CREATE TABLE checkout_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_checkout_session_id VARCHAR(255) UNIQUE NOT NULL,
    user_id UUID,
    mode VARCHAR(50) NOT NULL,
    status VARCHAR(50) NOT NULL,
    currency VARCHAR(3),
    amount_total BIGINT,
    customer_email VARCHAR(255),
    success_url TEXT NOT NULL,
    cancel_url TEXT NOT NULL,
    payment_intent_id VARCHAR(255),
    subscription_id UUID,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Foreign key constraints
    CONSTRAINT fk_checkout_sessions_user_id 
        FOREIGN KEY (user_id) 
        REFERENCES users(id) 
        ON DELETE SET NULL,
    CONSTRAINT fk_checkout_sessions_subscription_id 
        FOREIGN KEY (subscription_id) 
        REFERENCES subscriptions(id) 
        ON DELETE SET NULL,
        
    -- Check constraints
    CONSTRAINT ck_checkout_sessions_mode CHECK (
        mode IN ('payment', 'setup', 'subscription')
    ),
    CONSTRAINT ck_checkout_sessions_status CHECK (
        status IN ('open', 'complete', 'expired')
    ),
    CONSTRAINT ck_checkout_sessions_currency CHECK (
        currency IS NULL OR LENGTH(currency) = 3
    )
);

-- Indexes for checkout_sessions
CREATE INDEX idx_checkout_sessions_stripe_checkout_session_id ON checkout_sessions(stripe_checkout_session_id);
CREATE INDEX idx_checkout_sessions_user_id ON checkout_sessions(user_id);
CREATE INDEX idx_checkout_sessions_status ON checkout_sessions(status);
CREATE INDEX idx_checkout_sessions_expires_at ON checkout_sessions(expires_at);

----------------------------------------------------------------
-- Create webhook_events table
----------------------------------------------------------------
CREATE TABLE webhook_events (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    stripe_event_id VARCHAR(255) UNIQUE NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    api_version VARCHAR(20),
    data JSONB NOT NULL,
    processed BOOLEAN DEFAULT false,
    processed_at TIMESTAMP WITH TIME ZONE,
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
    
    -- Check constraints
    CONSTRAINT ck_webhook_events_retry_count CHECK (retry_count >= 0)
);

-- Indexes for webhook_events
CREATE INDEX idx_webhook_events_stripe_event_id ON webhook_events(stripe_event_id);
CREATE INDEX idx_webhook_events_event_type ON webhook_events(event_type);
CREATE INDEX idx_webhook_events_processed ON webhook_events(processed);
CREATE INDEX idx_webhook_events_created_at ON webhook_events(created_at);

----------------------------------------------------------------
-- Add triggers to automatically update updated_at timestamp
----------------------------------------------------------------
CREATE TRIGGER update_products_updated_at 
    BEFORE UPDATE ON products 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_prices_updated_at 
    BEFORE UPDATE ON prices 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_subscriptions_updated_at 
    BEFORE UPDATE ON subscriptions 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_subscription_items_updated_at 
    BEFORE UPDATE ON subscription_items 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_invoices_updated_at 
    BEFORE UPDATE ON invoices 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payments_updated_at 
    BEFORE UPDATE ON payments 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_checkout_sessions_updated_at 
    BEFORE UPDATE ON checkout_sessions 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_webhook_events_updated_at 
    BEFORE UPDATE ON webhook_events 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

----------------------------------------------------------------
-- Add comments for documentation
----------------------------------------------------------------
COMMENT ON COLUMN users.stripe_customer_id IS 'Stripe customer ID for billing integration';

COMMENT ON TABLE products IS 'Stripe products available for purchase';
COMMENT ON COLUMN products.stripe_product_id IS 'Stripe product ID from Stripe API';
COMMENT ON COLUMN products.metadata IS 'Additional product metadata as JSON';

COMMENT ON TABLE prices IS 'Pricing information for products';
COMMENT ON COLUMN prices.stripe_price_id IS 'Stripe price ID from Stripe API';
COMMENT ON COLUMN prices.unit_amount IS 'Price in smallest currency unit (e.g., cents)';
COMMENT ON COLUMN prices.recurring_interval IS 'Billing interval for recurring prices';
COMMENT ON COLUMN prices.tiers IS 'Tiered pricing structure for volume-based pricing';

COMMENT ON TABLE subscriptions IS 'User subscriptions to products';
COMMENT ON COLUMN subscriptions.stripe_subscription_id IS 'Stripe subscription ID from Stripe API';
COMMENT ON COLUMN subscriptions.status IS 'Current subscription status';
COMMENT ON COLUMN subscriptions.cancel_at_period_end IS 'Whether subscription will cancel at period end';

COMMENT ON TABLE subscription_items IS 'Individual items within a subscription';
COMMENT ON COLUMN subscription_items.stripe_subscription_item_id IS 'Stripe subscription item ID';
COMMENT ON COLUMN subscription_items.quantity IS 'Quantity of the subscription item';

COMMENT ON TABLE invoices IS 'Billing invoices for subscriptions and one-time payments';
COMMENT ON COLUMN invoices.stripe_invoice_id IS 'Stripe invoice ID from Stripe API';
COMMENT ON COLUMN invoices.amount_due IS 'Total amount due in smallest currency unit';
COMMENT ON COLUMN invoices.hosted_invoice_url IS 'Stripe-hosted invoice URL';

COMMENT ON TABLE payments IS 'Payment intents and payment processing';
COMMENT ON COLUMN payments.stripe_payment_intent_id IS 'Stripe PaymentIntent ID';
COMMENT ON COLUMN payments.client_secret IS 'Client secret for frontend payment confirmation';

COMMENT ON TABLE checkout_sessions IS 'Stripe Checkout sessions for payment flows';
COMMENT ON COLUMN checkout_sessions.stripe_checkout_session_id IS 'Stripe Checkout Session ID';
COMMENT ON COLUMN checkout_sessions.mode IS 'Checkout mode: payment, setup, or subscription';

COMMENT ON TABLE webhook_events IS 'Stripe webhook events for processing';
COMMENT ON COLUMN webhook_events.stripe_event_id IS 'Stripe event ID to prevent duplicate processing';
COMMENT ON COLUMN webhook_events.processed IS 'Whether the webhook has been successfully processed';
COMMENT ON COLUMN webhook_events.retry_count IS 'Number of processing retry attempts';