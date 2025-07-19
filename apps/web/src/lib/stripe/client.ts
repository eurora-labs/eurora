import { loadStripe, type Stripe } from '@stripe/stripe-js';
import type {
	StripeConfig,
	StripeApiError,
	CreateCustomerRequest,
	CustomerResponse,
	CreateCheckoutSessionRequest,
	CheckoutSessionResponse,
	CreatePaymentIntentRequest,
	PaymentIntentResponse,
	CreateSubscriptionRequest,
	SubscriptionResponse,
	ProductResponse,
	HealthResponse,
} from './types.js';

class StripeApiClient {
	private baseUrl: string;
	private stripe: Promise<Stripe | null>;
	private publishableKey: string;

	constructor(config: StripeConfig) {
		this.baseUrl = config.apiUrl;
		this.publishableKey = config.publishableKey;
		this.stripe = loadStripe(config.publishableKey);
	}

	private async request<T>(endpoint: string, options: RequestInit = {}): Promise<T> {
		const url = `${this.baseUrl}${endpoint}`;

		const response = await fetch(url, {
			headers: {
				'Content-Type': 'application/json',
				...options.headers,
			},
			...options,
		});

		const data = await response.json();

		if (!response.ok) {
			throw new Error((data as StripeApiError).error?.message || 'API request failed');
		}

		return data as T;
	}

	async getStripe(): Promise<Stripe | null> {
		return this.stripe;
	}

	getPublishableKey(): string {
		return this.publishableKey;
	}

	async createCustomer(request: CreateCustomerRequest): Promise<CustomerResponse> {
		return this.request<CustomerResponse>('/customers', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async getCustomer(customerId: string): Promise<CustomerResponse> {
		return this.request<CustomerResponse>(`/customers/${customerId}`);
	}

	async createCheckoutSession(
		request: CreateCheckoutSessionRequest,
	): Promise<CheckoutSessionResponse> {
		return this.request<CheckoutSessionResponse>('/checkout-sessions', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async createPaymentIntent(request: CreatePaymentIntentRequest): Promise<PaymentIntentResponse> {
		return this.request<PaymentIntentResponse>('/payment-intents', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async createSubscription(request: CreateSubscriptionRequest): Promise<SubscriptionResponse> {
		return this.request<SubscriptionResponse>('/subscriptions', {
			method: 'POST',
			body: JSON.stringify(request),
		});
	}

	async getProduct(productId: string): Promise<ProductResponse> {
		return this.request<ProductResponse>(`/products/${productId}`);
	}

	async healthCheck(): Promise<HealthResponse> {
		return this.request<HealthResponse>('/health');
	}

	/**
	 * Redirect to Stripe Checkout using the session URL
	 */
	async redirectToCheckout(sessionId: string): Promise<{ error?: any }> {
		const stripe = await this.getStripe();
		if (!stripe) {
			throw new Error('Stripe not loaded');
		}

		return stripe.redirectToCheckout({ sessionId });
	}

	/**
	 * Create checkout session and redirect
	 */
	async createAndRedirectToCheckout(request: CreateCheckoutSessionRequest): Promise<void> {
		const session = await this.createCheckoutSession(request);

		if (session.url) {
			// For server-side created sessions, we can redirect directly to the URL
			window.location.href = session.url;
		} else {
			// For client-side sessions, use Stripe's redirectToCheckout
			const result = await this.redirectToCheckout(session.id);
			if (result.error) {
				throw new Error(result.error.message);
			}
		}
	}

	/**
	 * Create Stripe Elements instance for custom payment forms
	 */
	async createElements(options?: any): Promise<any> {
		const stripe = await this.getStripe();
		if (!stripe) {
			throw new Error('Stripe not loaded');
		}
		return stripe.elements(options);
	}

	/**
	 * Confirm payment with payment method
	 */
	async confirmPayment(clientSecret: string, paymentMethod: any): Promise<any> {
		const stripe = await this.getStripe();
		if (!stripe) {
			throw new Error('Stripe not loaded');
		}
		return stripe.confirmPayment({
			clientSecret,
			confirmParams: paymentMethod,
		});
	}
}

// Default configuration - these should be set at build time or runtime
const DEFAULT_CONFIG: StripeConfig = {
	publishableKey:
		'pk_test_51RmZGBFbfKULj0cN4XGX2nuvoWaItaO1c7iXxK91UjnMqhTiCAbLQf3UDt6srATfvM5NmAdq0czNL7LYidMDKei200nypFLBOX', // This should be replaced with actual key
	apiUrl: 'http://localhost:3003', // This should point to the Rust backend
};

// Create a singleton instance
let stripeClient: StripeApiClient | null = null;

export function getStripeClient(config?: Partial<StripeConfig>): StripeApiClient {
	if (!stripeClient) {
		const finalConfig = { ...DEFAULT_CONFIG, ...config };
		stripeClient = new StripeApiClient(finalConfig);
	}
	return stripeClient;
}

export { StripeApiClient };
