<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { consumeAppRedirectUri } from '$lib/auth/redirect-uri';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';

	const auth = inject(AUTH_SERVICE);

	onMount(async () => {
		try {
			// Apple's form-post flow set the session cookies via a
			// backend-issued 303. The SPA's `ready` promise resolved
			// before this page mounted, so the local `isAuthenticated`
			// state still says "logged out". Re-probe `/auth/me` to
			// pick up the new session.
			await auth.rehydrate();

			const paired = page.url.searchParams.get('paired') === '1';
			if (paired) {
				const redirectUri = consumeAppRedirectUri();
				if (redirectUri) {
					window.location.href = redirectUri;
					return;
				}
			}

			// `loginToken` / `challengeMethod` may have been set on
			// the login page before bouncing through Apple. Clear
			// them now — the pairing token was consumed server-side
			// at URL-issue time, so there is no second-pass use for
			// the SPA copy.
			sessionStorage.removeItem('loginToken');
			sessionStorage.removeItem('challengeMethod');

			const redirect = sessionStorage.getItem('postLoginRedirect') || '/';
			sessionStorage.removeItem('postLoginRedirect');
			goto(redirect);
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.oauth', provider: 'apple' } });
			goto('/login?error=oauth_failed');
		}
	});
</script>

<svelte:head>
	<title>Signing in… - Eurora Labs</title>
</svelte:head>

<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected…</p>
</div>
