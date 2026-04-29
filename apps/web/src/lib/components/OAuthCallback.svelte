<script lang="ts">
	import { goto } from '$app/navigation';
	import { AUTH_SERVICE, type OAuthProvider } from '$lib/services/auth-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';

	let { provider }: { provider: OAuthProvider } = $props();

	const auth = inject(AUTH_SERVICE);

	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const error = query.get('error');

		if (error) {
			Sentry.captureMessage('OAuth provider returned error', {
				level: 'warning',
				tags: { area: 'auth.oauth', provider },
				extra: { error, description: query.get('error_description') },
			});
			goto('/login?error=oauth_failed');
			return;
		}

		const code = query.get('code');
		const state = query.get('state');

		if (!code || !state) {
			Sentry.captureMessage('OAuth callback missing code or state', {
				level: 'warning',
				tags: { area: 'auth.oauth', provider },
			});
			goto('/login?error=invalid_callback');
			return;
		}

		const loginToken = sessionStorage.getItem('loginToken') ?? undefined;
		const challengeMethod = sessionStorage.getItem('challengeMethod') ?? undefined;
		if (loginToken) sessionStorage.removeItem('loginToken');
		if (challengeMethod) sessionStorage.removeItem('challengeMethod');

		try {
			await auth.loginWithOAuth(provider, code, state, { loginToken, challengeMethod });

			if (loginToken) {
				const deviceRedirectUri = sessionStorage.getItem('deviceRedirectUri');
				if (deviceRedirectUri) {
					sessionStorage.removeItem('deviceRedirectUri');
					window.location.href = deviceRedirectUri;
					return;
				}
			}

			const redirect = sessionStorage.getItem('postLoginRedirect') || '/';
			sessionStorage.removeItem('postLoginRedirect');
			goto(redirect);
		} catch (err) {
			Sentry.captureException(err, { tags: { area: 'auth.oauth', provider } });
			goto('/login?error=token_exchange_failed');
		}
	});
</script>

<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
