<script lang="ts">
	import { LoginRequestSchema, Provider } from '@eurora/proto/auth_service';
	import { create } from '@bufbuild/protobuf';
	import { onMount } from 'svelte';
	import { authService } from '$lib/services/auth-service';
	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const code = query.get('code');
		const state = query.get('state');
		if (!code || !state) {
			console.error('No code or state found in query parameters');
			return;
		}

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
	});
</script>

<!-- A simple wait to be redirected page -->
<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
