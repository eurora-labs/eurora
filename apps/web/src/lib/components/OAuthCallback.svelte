<script lang="ts">
	import { goto } from '$app/navigation';
	import { AUTH_SERVICE } from '$lib/services/auth-service.js';
	import { auth } from '$lib/stores/auth.js';
	import { create } from '@bufbuild/protobuf';
	import { inject } from '@eurora/shared/context';
	import { LoginRequestSchema, type Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import * as Sentry from '@sentry/sveltekit';
	import { onMount } from 'svelte';

	let { provider }: { provider: Provider } = $props();

	const authService = inject(AUTH_SERVICE);

	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const error = query.get('error');

		if (error) {
			Sentry.captureMessage('OAuth provider returned error', {
				level: 'warning',
				tags: { area: 'auth.oauth', provider: String(provider) },
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
				tags: { area: 'auth.oauth', provider: String(provider) },
			});
			goto('/login?error=invalid_callback');
			return;
		}

		const loginToken = sessionStorage.getItem('loginToken') ?? undefined;
		const challengeMethod = sessionStorage.getItem('challengeMethod') ?? undefined;
		if (loginToken) sessionStorage.removeItem('loginToken');
		if (challengeMethod) sessionStorage.removeItem('challengeMethod');

		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						provider,
						code,
						state,
						loginToken,
						challengeMethod,
					},
					case: 'thirdParty',
				},
			});

			const tokens = await authService.login(loginData);
			auth.login(tokens);

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
		} catch (error) {
			Sentry.captureException(error, {
				tags: { area: 'auth.oauth', provider: String(provider) },
			});
			goto('/login?error=token_exchange_failed');
		}
	});
</script>

<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
