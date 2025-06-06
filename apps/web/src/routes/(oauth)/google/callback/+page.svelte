<script lang="ts">
	import { LoginRequestSchema, Provider } from '@eurora/proto/auth_service';
	import { create } from '@bufbuild/protobuf';
	import { onMount } from 'svelte';
	import { authService } from '$lib/services/auth-service';
	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const error = query.get('error');

		if (error) {
			console.error('OAuth error:', error, query.get('error_description'));
			window.location.href = '/login?error=oauth_failed';
			return;
		}

		const code = query.get('code');
		const state = query.get('state');

		if (!code || !state) {
			console.error('Missing required OAuth parameters');
			window.location.href = '/login?error=invalid_callback';
			return;
		}

		// TODO: Validate state parameter against stored value for CSRF protection

		try {
			const loginData = create(LoginRequestSchema, {
				credential: {
					value: {
						provider: Provider.GOOGLE,
						code,
						state,
					},
					case: 'thirdParty',
				},
			});

			const tokens = await authService.login(loginData);

			console.log('Tokens:', tokens);
		} catch (error) {
			console.error('Token exchange failed:', error);
			window.location.href = '/login?error=token_exchange_failed';
		}
	});
</script>

<!-- A simple wait to be redirected page -->
<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
