<script lang="ts">
	import { goto } from '$app/navigation';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import { CONFIG_SERVICE } from '@eurora/shared/config/config-service';
	import { inject } from '@eurora/shared/context';
	import { Button, type ButtonProps } from '@eurora/ui/components/button/index';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { Snippet } from 'svelte';

	const auth = inject(AUTH_SERVICE);
	const { restApiUrl: REST_API_URL } = inject(CONFIG_SERVICE);

	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID ?? '';
	const CHECKOUT_REDIRECT = '/pricing?checkout=true';

	let {
		class: className = '',
		variant = 'default' as ButtonProps['variant'],
		autoTrigger = false,
		children,
		...restProps
	}: ButtonProps & { children?: Snippet; autoTrigger?: boolean } = $props();

	let loading = $state(false);
	let resending = $state(false);

	onMount(() => {
		if (autoTrigger) handleGetPro();
	});

	async function handleGetPro() {
		if (!auth.isAuthenticated) {
			goto('/login?redirect=' + encodeURIComponent(CHECKOUT_REDIRECT));
			return;
		}

		loading = true;

		try {
			if (!(await auth.ensureValidToken())) {
				goto('/login?redirect=' + encodeURIComponent(CHECKOUT_REDIRECT));
				return;
			}

			// Gate the checkout call on the verification flag from the JWT.
			// The backend enforces this too, but short-circuiting here gives the
			// user a useful next step (resend) instead of a generic toast.
			if (!auth.user?.emailVerified) {
				promptEmailVerification();
				return;
			}

			const res = await fetch(`${REST_API_URL}/payment/checkout`, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
					Authorization: `Bearer ${auth.accessToken}`,
				},
				body: JSON.stringify({
					price_id: STRIPE_PRO_PRICE_ID,
				}),
			});

			const body = (await res.json().catch(() => null)) as {
				url?: string;
				error?: string;
			} | null;

			if (!res.ok) {
				if (
					res.status === 403 &&
					typeof body?.error === 'string' &&
					/email/i.test(body.error)
				) {
					promptEmailVerification();
					return;
				}
				throw new Error(body?.error ?? `Checkout failed (${res.status})`);
			}

			if (!body?.url) {
				throw new Error('Checkout response missing redirect URL');
			}
			window.location.href = body.url;
		} catch (err) {
			Sentry.captureException(err, {
				tags: { area: 'payment.checkout' },
				extra: { priceId: STRIPE_PRO_PRICE_ID },
			});
			toast.error(
				err instanceof Error ? err.message : 'Something went wrong. Please try again.',
			);
		} finally {
			loading = false;
		}
	}

	function promptEmailVerification() {
		const email = auth.user?.email;
		toast.message('Verify your email to continue', {
			description: email
				? `We sent a verification link to ${email}. Click the link, then try again.`
				: 'We sent you a verification link. Click it, then try again.',
			action: {
				label: 'Resend email',
				onClick: () => {
					void resendVerificationEmail();
				},
			},
			duration: 10_000,
		});
	}

	async function resendVerificationEmail() {
		if (resending) return;
		resending = true;
		try {
			await auth.resendVerificationEmail();
			toast.success('Verification email sent. Check your inbox.');
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.resend-verification' } });
			const message =
				err instanceof Error ? err.message : 'Could not send verification email.';
			toast.error(message);
		} finally {
			resending = false;
		}
	}
</script>

<Button {variant} class={className} onclick={handleGetPro} disabled={loading} {...restProps}>
	{#if loading}
		<Loader2Icon class="mr-2 h-4 w-4 animate-spin" />
		Redirecting...
	{:else}
		{@render children?.()}
	{/if}
</Button>
