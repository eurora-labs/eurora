<script lang="ts">
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth.js';
	import { create } from '@bufbuild/protobuf';
	import { LoginRequestSchema, Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import { authService } from '@eurora/shared/services/auth-service';
	import { onMount } from 'svelte';

	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const error = query.get('error');

		if (error) {
			console.error('OAuth error:', error, query.get('error_description'));
			goto('/login?error=oauth_failed');
			return;
		}

		const code = query.get('code');
		const state = query.get('state');

		if (!code || !state) {
			console.error('Missing required OAuth parameters');
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
						provider: Provider.GOOGLE,
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

			goto('/settings');
		} catch (error) {
			console.error('Token exchange failed:', error);
			goto('/login?error=token_exchange_failed');
		}
	});
</script>

<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
