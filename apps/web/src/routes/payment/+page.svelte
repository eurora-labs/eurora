<script>
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { loadStripe } from '@stripe/stripe-js';
	import { PUBLIC_STRIPE_KEY } from '$env/static/public';
	import { Elements, PaymentElement, LinkAuthenticationElement, Address } from 'svelte-stripe';

	let stripe = null;
	let clientSecret = null;
	let error = null;
	let elements;
	let processing = false;

	onMount(async () => {
		stripe = await loadStripe(PUBLIC_STRIPE_KEY);

		// create payment intent server side
		clientSecret = await createPaymentIntent();
	});

	async function createPaymentIntent() {
		const response = await fetch('/payment/payment-intent', {
			method: 'POST',
			headers: {
				'content-type': 'application/json',
			},
			body: JSON.stringify({}),
		});
		const { clientSecret } = await response.json();

		return clientSecret;
	}

	async function submit() {
		// avoid processing duplicates
		if (processing) return;

		processing = true;

		// confirm payment with stripe
		const result = await stripe.confirmPayment({
			elements,
			redirect: 'if_required',
		});

		// log results, for debugging
		console.log({ result });

		if (result.error) {
			// payment failed, notify user
			error = result.error;
			processing = false;
		} else {
			// payment succeeded, redirect to "thank you" page
			goto('/examples/payment-element/thanks');
		}
	}
</script>

<h1>Payment Element Example</h1>

<nav>
	<a
		href="https://github.com/joshnuss/svelte-stripe/tree/main/src/routes/examples/payment-element"
		>View code</a
	>
</nav>

{#if error}
	<p class="error">{error.message} Please try again.</p>
{/if}

{#if clientSecret}
	<Elements
		{stripe}
		{clientSecret}
		theme="flat"
		labels="floating"
		variables={{ colorPrimary: '#7c4dff' }}
		rules={{ '.Input': { border: 'solid 1px #0002' } }}
		bind:elements
	>
		<form on:submit|preventDefault={submit}>
			<LinkAuthenticationElement />
			<PaymentElement />
			<Address mode="billing" />

			<button disabled={processing}>
				{#if processing}
					Processing...
				{:else}
					Pay
				{/if}
			</button>
		</form>
	</Elements>
{:else}
	Loading...
{/if}

<style>
	.error {
		margin: 2rem 0 0;
		color: tomato;
	}

	form {
		display: flex;
		flex-direction: column;
		margin: 2rem 0;
		gap: 10px;
	}

	button {
		margin: 1rem 0;
		padding: 1rem;
		border: solid 1px #ccc;
		border-radius: 5px;
		background: var(--link-color);
		color: white;
		font-size: 1.2rem;
	}
</style>
