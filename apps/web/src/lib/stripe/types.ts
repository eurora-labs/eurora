// Stripe API Types for the web app integration

export interface StripeConfig {
	publishableKey: string;
	apiUrl: string;
}

export interface StripeApiError {
	error: {
		message: string;
		type: string;
		code?: string;
		param?: string;
	};
}

// Customer Types
export interface CreateCustomerRequest {
	email: string;
	name?: string;
	description?: string;
	metadata?: Record<string, any>;
}

export interface CustomerResponse {
	id: string;
	email: string;
	name?: string;
	description?: string;
	created: string;
	metadata: Record<string, any>;
}

// Product Types
export interface CreateProductRequest {
	name: string;
	description?: string;
	active?: boolean;
	metadata?: Record<string, any>;
}

export interface ProductResponse {
	id: string;
	name: string;
	description?: string;
	active: boolean;
	created: string;
	metadata: Record<string, any>;
}

// Price Types
export interface CreatePriceRequest {
	product_id: string;
	unit_amount: number;
	currency: string;
	recurring?: RecurringRequest;
	metadata?: Record<string, any>;
}

export interface RecurringRequest {
	interval: 'day' | 'week' | 'month' | 'year';
	interval_count?: number;
}

export interface PriceResponse {
	id: string;
	product_id: string;
	unit_amount: number;
	currency: string;
	recurring?: RecurringResponse;
	active: boolean;
	created: string;
	metadata: Record<string, any>;
}

export interface RecurringResponse {
	interval: string;
	interval_count: number;
}

// Subscription Types
export interface CreateSubscriptionRequest {
	customer_id: string;
	price_id: string;
	quantity?: number;
	trial_period_days?: number;
	metadata?: Record<string, any>;
}

export interface SubscriptionResponse {
	id: string;
	customer_id: string;
	status: SubscriptionStatus;
	current_period_start: string;
	current_period_end: string;
	trial_start?: string;
	trial_end?: string;
	cancel_at_period_end: boolean;
	created: string;
	metadata: Record<string, any>;
}

export type SubscriptionStatus =
	| 'incomplete'
	| 'incomplete_expired'
	| 'trialing'
	| 'active'
	| 'past_due'
	| 'canceled'
	| 'unpaid'
	| 'paused';

// Payment Intent Types
export interface CreatePaymentIntentRequest {
	amount: number;
	currency: string;
	customer_id?: string;
	description?: string;
	metadata?: Record<string, any>;
	automatic_payment_methods?: boolean;
}

export interface PaymentIntentResponse {
	id: string;
	amount: number;
	currency: string;
	status: PaymentIntentStatus;
	client_secret: string;
	customer_id?: string;
	description?: string;
	created: string;
	metadata: Record<string, any>;
}

export type PaymentIntentStatus =
	| 'requires_payment_method'
	| 'requires_confirmation'
	| 'requires_action'
	| 'processing'
	| 'requires_capture'
	| 'canceled'
	| 'succeeded';

// Checkout Session Types
export interface CreateCheckoutSessionRequest {
	customer_id?: string;
	customer_email?: string;
	line_items: CheckoutLineItem[];
	mode: CheckoutMode;
	success_url: string;
	cancel_url: string;
	metadata?: Record<string, any>;
}

export interface CheckoutLineItem {
	price_id: string;
	quantity: number;
}

export type CheckoutMode = 'payment' | 'setup' | 'subscription';

export interface CheckoutSessionResponse {
	id: string;
	url?: string;
	customer_id?: string;
	mode: string;
	status: string;
	success_url: string;
	cancel_url: string;
	created: string;
	metadata: Record<string, any>;
}

// Pricing Plan Types (for the web app)
export interface PricingPlan {
	id: string;
	name: string;
	description: string;
	price: number;
	currency: string;
	interval?: 'month' | 'year';
	features: string[];
	popular?: boolean;
	priceId: string; // Stripe price ID
}

// Health Check Types
export interface HealthResponse {
	status: string;
	timestamp: string;
	version: string;
}

// Webhook Event Types (for reference, handled by Rust backend)
export interface WebhookEvent {
	id: string;
	object: 'event';
	api_version: string;
	created: number;
	data: {
		object: any;
		previous_attributes?: any;
	};
	livemode: boolean;
	pending_webhooks: number;
	request: {
		id: string;
		idempotency_key?: string;
	};
	type: WebhookEventType;
}

export type WebhookEventType =
	| 'customer.created'
	| 'customer.updated'
	| 'customer.deleted'
	| 'customer.subscription.created'
	| 'customer.subscription.updated'
	| 'customer.subscription.deleted'
	| 'invoice.payment_succeeded'
	| 'invoice.payment_failed'
	| 'payment_intent.succeeded'
	| 'payment_intent.payment_failed'
	| 'checkout.session.completed'
	| 'checkout.session.expired';

// Service Response Types
export interface ServiceResponse<T> {
	data?: T;
	error?: string;
	success: boolean;
}

// Subscription Management Types
export interface SubscriptionDetails {
	id: string;
	status: SubscriptionStatus;
	plan: {
		name: string;
		amount: number;
		currency: string;
		interval: string;
	};
	current_period_start: string;
	current_period_end: string;
	cancel_at_period_end: boolean;
	customer: {
		email: string;
	};
}

// Payment Method Types
export interface PaymentMethod {
	id: string;
	type: 'card';
	card: {
		brand: string;
		last4: string;
		exp_month: number;
		exp_year: number;
	};
}

// Error Types
export class StripeServiceError extends Error {
	constructor(
		message: string,
		public code?: string,
		public type?: string,
	) {
		super(message);
		this.name = 'StripeServiceError';
	}
}

// Utility Types
export interface StripeElementsOptions {
	clientSecret?: string;
	appearance?: {
		theme?: 'stripe' | 'night' | 'flat';
		variables?: Record<string, string>;
	};
}

export interface ConfirmPaymentData {
	elements: any;
	confirmParams?: {
		return_url?: string;
		payment_method_data?: any;
	};
	redirect?: 'if_required' | 'always';
}
