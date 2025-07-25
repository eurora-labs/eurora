<script lang="ts">
	import { LoginRequestSchema, Provider } from '@eurora/shared/proto/auth_service_pb.js';
	import { create } from '@bufbuild/protobuf';
	import { onMount } from 'svelte';
	import { authService } from '@eurora/shared/services/auth-service';
	import { goto } from '$app/navigation';

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

		// State validation is now handled by the backend auth service
		// The backend will validate the state parameter against the stored OAuth state
		// and return an error if the state is invalid or expired
		const loginToken = sessionStorage.getItem('loginToken') ?? undefined;
		const challengeMethod = sessionStorage.getItem('challengeMethod') ?? undefined;
		if (loginToken) sessionStorage.removeItem('loginToken');
		if (challengeMethod) sessionStorage.removeItem('challengeMethod');

		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						provider: Provider.GITHUB,
						code,
						state,
						loginToken,
						challengeMethod,
					},
					case: 'thirdParty',
				},
			});

			const tokens = await authService.login(loginData);

			console.log('Tokens:', tokens);

			// Store tokens in auth store
			// auth.login(tokens);

			// Redirect to home page
			goto('/');
		} catch (error) {
			console.error('Token exchange failed:', error);
			goto('/login?error=token_exchange_failed');
		}
	});
</script>

<!-- A simple wait to be redirected page -->
<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
