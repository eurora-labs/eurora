import { getStripeClient } from './client.js';
import type {
	CreateCheckoutSessionRequest,
	CheckoutSessionResponse,
	StripeConfig,
	PricingPlan,
} from './types.js';
import { browser } from '$app/environment';

// Define the pricing plans that match the current pricing page
export const PRICING_PLANS: PricingPlan[] = [
	{
		id: 'free',
		name: 'Free',
		description: 'Perfect for casual users',
		price: 0,
		currency: 'usd',
		features: [
			'Basic AI assistance',
			'Up to 20 queries per day',
			'Standard response time',
			'Desktop and mobile apps',
		],
		priceId: '', // Free plan doesn't need a price ID
	},
	{
		id: 'pro',
		name: 'Pro',
		description: 'For power users and professionals',
		price: 12,
		currency: 'usd',
		interval: 'month',
		features: [
			'Advanced AI assistance',
			'Unlimited queries',
			'Faster response time',
			'Desktop and mobile apps',
			'Access to premium AI models',
			'Priority email support',
		],
		popular: true,
		priceId: 'price_pro_monthly', // This should be replaced with actual Stripe price ID
	},
	{
		id: 'enterprise',
		name: 'Enterprise',
		description: 'For teams and organizations',
		price: 29,
		currency: 'usd',
		interval: 'month',
		features: [
			'Enterprise-grade AI assistance',
			'Unlimited queries with higher rate limits',
			'Fastest response time',
			'All apps and platforms',
			'Access to all AI models',
			'24/7 dedicated support',
			'Custom integrations & API access',
		],
		priceId: 'price_enterprise_monthly', // This should be replaced with actual Stripe price ID
	},
];

export class StripeService {
	private client;

	constructor(config?: Partial<StripeConfig>) {
		this.client = getStripeClient(config);
	}

	/**
	 * Create a checkout session for a subscription
	 */
	async createSubscriptionCheckout(
		priceId: string,
		options: {
			customerEmail?: string;
			customerId?: string;
			successUrl?: string;
			cancelUrl?: string;
			quantity?: number;
			metadata?: Record<string, any>;
		} = {},
	): Promise<CheckoutSessionResponse> {
		const {
			customerEmail,
			customerId,
			successUrl = `${window.location.origin}/payment/success`,
			cancelUrl = `${window.location.origin}/payment/cancel`,
			quantity = 1,
			metadata = {},
		} = options;

		const request: CreateCheckoutSessionRequest = {
			line_items: [
				{
					price_id: priceId,
					quantity,
				},
			],
			mode: 'subscription',
			success_url: successUrl,
			cancel_url: cancelUrl,
			metadata,
		};

		if (customerId) {
			request.customer_id = customerId;
		} else if (customerEmail) {
			request.customer_email = customerEmail;
		}

		return this.client.createCheckoutSession(request);
	}

	/**
	 * Create a one-time payment checkout session
	 */
	async createPaymentCheckout(
		priceId: string,
		options: {
			customerEmail?: string;
			customerId?: string;
			successUrl?: string;
			cancelUrl?: string;
			quantity?: number;
			metadata?: Record<string, any>;
		} = {},
	): Promise<CheckoutSessionResponse> {
		const {
			customerEmail,
			customerId,
			successUrl = `${window.location.origin}/payment/success`,
			cancelUrl = `${window.location.origin}/payment/cancel`,
			quantity = 1,
			metadata = {},
		} = options;

		const request: CreateCheckoutSessionRequest = {
			line_items: [
				{
					price_id: priceId,
					quantity,
				},
			],
			mode: 'payment',
			success_url: successUrl,
			cancel_url: cancelUrl,
			metadata,
		};

		if (customerId) {
			request.customer_id = customerId;
		} else if (customerEmail) {
			request.customer_email = customerEmail;
		}

		return this.client.createCheckoutSession(request);
	}

	/**
	 * Redirect to Stripe checkout
	 */
	async redirectToCheckout(
		priceId: string,
		mode: 'subscription' | 'payment' = 'subscription',
		options: {
			customerEmail?: string;
			customerId?: string;
			successUrl?: string;
			cancelUrl?: string;
			quantity?: number;
			metadata?: Record<string, any>;
		} = {},
	): Promise<void> {
		if (!browser) {
			throw new Error('redirectToCheckout can only be called in the browser');
		}

		try {
			const request: CreateCheckoutSessionRequest = {
				line_items: [
					{
						price_id: priceId,
						quantity: options.quantity || 1,
					},
				],
				mode,
				success_url: options.successUrl || `${window.location.origin}/payment/success`,
				cancel_url: options.cancelUrl || `${window.location.origin}/payment/cancel`,
				metadata: options.metadata || {},
			};

			if (options.customerId) {
				request.customer_id = options.customerId;
			} else if (options.customerEmail) {
				request.customer_email = options.customerEmail;
			}

			await this.client.createAndRedirectToCheckout(request);
		} catch (error) {
			console.error('Error redirecting to checkout:', error);
			throw error;
		}
	}

	/**
	 * Get a pricing plan by ID
	 */
	getPlan(planId: string): PricingPlan | undefined {
		return PRICING_PLANS.find((plan) => plan.id === planId);
	}

	/**
	 * Get all pricing plans
	 */
	getPlans(): PricingPlan[] {
		return PRICING_PLANS;
	}

	/**
	 * Format price for display
	 */
	formatPrice(amount: number, currency: string = 'usd'): string {
		return new Intl.NumberFormat('en-US', {
			style: 'currency',
			currency: currency.toUpperCase(),
			minimumFractionDigits: 0,
			maximumFractionDigits: 2,
		}).format(amount);
	}

	/**
	 * Check if the Stripe service is healthy
	 */
	async healthCheck(): Promise<boolean> {
		try {
			const health = await this.client.healthCheck();
			return health.status === 'healthy';
		} catch (error) {
			console.error('Stripe service health check failed:', error);
			return false;
		}
	}
}

// Create a singleton instance
let stripeService: StripeService | null = null;

export function getStripeService(config?: Partial<StripeConfig>): StripeService {
	if (!stripeService) {
		stripeService = new StripeService(config);
	}
	return stripeService;
}

export { StripeService as default };
