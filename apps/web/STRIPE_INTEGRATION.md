# Stripe Integration Documentation

This document describes the Stripe integration between the static SvelteKit web app and the Rust backend service.

## Architecture Overview

The integration follows a client-server architecture where:

- **Web App (SvelteKit)**: Static frontend that handles UI and redirects to Stripe Checkout
- **Rust Backend**: Handles all server-side Stripe operations, webhooks, and database interactions

## Files Structure

```
apps/web/src/lib/stripe/
├── client.ts          # Stripe API client for communicating with Rust backend
├── service.ts         # High-level service layer with business logic
└── types.ts           # TypeScript type definitions

apps/web/src/routes/
├── (marketing)/pricing/+page.svelte    # Pricing page with checkout buttons
├── payment/success/+page.svelte        # Payment success page
├── payment/cancel/+page.svelte         # Payment cancellation page
└── subscription/+page.svelte           # Subscription management page
```

## Configuration

### 1. Stripe Configuration

The web app needs to be configured with:

```typescript
// In your Stripe service initialization
const stripeService = getStripeService({
	publishableKey: 'pk_live_...', // Your Stripe publishable key
	apiUrl: 'https://your-backend.com', // Your Rust backend URL
});
```

### 2. Environment Variables

Since the web app is static, configuration should be done at build time or through runtime configuration:

```javascript
// In your app configuration
const STRIPE_CONFIG = {
	publishableKey: 'pk_live_...', // Replace with actual publishable key
	apiUrl: 'https://api.yourapp.com', // Replace with your Rust backend URL
};
```

### 3. Rust Backend Configuration

Ensure your Rust backend is configured with:

- Stripe secret key
- Webhook endpoint secret
- Database connection
- CORS settings to allow requests from your web app domain

## Usage Examples

### 1. Basic Checkout Flow

```svelte
<script>
	import { getStripeService } from '$lib/stripe/service.js';

	const stripeService = getStripeService();

	async function handleSubscribe() {
		try {
			await stripeService.redirectToCheckout('price_1234567890', 'subscription');
		} catch (error) {
			console.error('Checkout error:', error);
		}
	}
</script>

<button on:click={handleSubscribe}>Subscribe Now</button>
```

### 2. Custom Payment Form (using svelte-stripe)

```svelte
<script>
	import { Elements, PaymentElement } from 'svelte-stripe';
	import { getStripeClient } from '$lib/stripe/client.js';

	const stripeClient = getStripeClient();
	let stripe;
	let elements;

	onMount(async () => {
		stripe = await stripeClient.getStripe();
	});
</script>

{#if stripe}
	<Elements {stripe} bind:elements>
		<PaymentElement />
		<!-- Add form submission logic -->
	</Elements>
{/if}
```

## API Endpoints

The web app communicates with these Rust backend endpoints:

### Customer Management

- `POST /customers` - Create customer
- `GET /customers/{id}` - Get customer details

### Products & Prices

- `POST /products` - Create product
- `GET /products/{id}` - Get product details
- `POST /prices` - Create price

### Subscriptions

- `POST /subscriptions` - Create subscription
- `GET /subscriptions/{id}` - Get subscription details

### Checkout Sessions

- `POST /checkout-sessions` - Create checkout session

### Payment Intents

- `POST /payment-intents` - Create payment intent

### Health Check

- `GET /health` - Service health check

## Webhook Handling

Webhooks are handled entirely by the Rust backend at `/webhooks/stripe`. The backend should:

1. Verify webhook signatures
2. Process events (subscription updates, payment confirmations, etc.)
3. Update database records
4. Send notifications if needed

Common webhook events to handle:

- `checkout.session.completed`
- `customer.subscription.created`
- `customer.subscription.updated`
- `customer.subscription.deleted`
- `invoice.payment_succeeded`
- `invoice.payment_failed`

## Deployment Configuration

### 1. Web App Deployment

Since the web app is static, ensure:

- Stripe publishable key is configured for production
- API URL points to your production Rust backend
- Success/cancel URLs are configured correctly

### 2. Rust Backend Deployment

Ensure the backend:

- Has proper CORS configuration for your web app domain
- Uses HTTPS in production
- Has webhook endpoint configured in Stripe dashboard
- Environment variables are properly set

### 3. Stripe Dashboard Configuration

Configure in your Stripe dashboard:

- Webhook endpoint: `https://your-backend.com/webhooks/stripe`
- Webhook events: Select relevant events for your use case
- API keys: Use live keys for production

## Security Considerations

1. **API Keys**: Never expose secret keys in the frontend
2. **Webhook Security**: Always verify webhook signatures in the backend
3. **HTTPS**: Use HTTPS for all production endpoints
4. **CORS**: Configure CORS properly to only allow your domain
5. **Input Validation**: Validate all inputs in the backend

## Testing

### Local Development

1. Start the Rust backend:

    ```bash
    cd crates/backend/eur-stripe-service
    cargo run
    ```

2. Start the web app:

    ```bash
    cd apps/web
    npm run dev
    ```

3. Use Stripe test keys and test card numbers

### Integration Testing

Test the complete flow:

1. Navigate to pricing page
2. Click subscription button
3. Complete checkout with test card
4. Verify success page
5. Check subscription in backend/database

## Troubleshooting

### Common Issues

1. **CORS Errors**: Ensure backend CORS is configured for your web app domain
2. **Webhook Failures**: Check webhook signature verification in backend
3. **Checkout Redirect Issues**: Verify success/cancel URLs are accessible
4. **API Connection Issues**: Check network connectivity and API URL configuration

### Debug Mode

Enable debug logging in both frontend and backend to trace issues:

```typescript
// Frontend debugging
console.log('Stripe config:', stripeConfig);
console.log('Checkout session:', session);
```

```rust
// Backend debugging
tracing::debug!("Processing webhook: {:?}", event);
```

## Support

For issues related to:

- Stripe API: Check [Stripe Documentation](https://stripe.com/docs)
- SvelteKit: Check [SvelteKit Documentation](https://kit.svelte.dev/)
- Integration issues: Check application logs and network requests
