import { json } from '@sveltejs/kit';
import Stripe from 'stripe';
import { env } from '$env/dynamic/private';
import { building } from '$app/environment';

let stripe: Stripe;
if (!building) {
	stripe = new Stripe(env.SECRET_STRIPE_KEY);
}

export async function POST() {
	const paymentIntent = await stripe.paymentIntents.create({
		amount: 2000,
		currency: 'usd',
		automatic_payment_methods: {
			enabled: true,
		},
	});

	return json({
		clientSecret: paymentIntent.client_secret,
	});
}
