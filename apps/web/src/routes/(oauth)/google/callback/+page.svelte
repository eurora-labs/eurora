<script lang="ts">
	import { onMount } from 'svelte';
	import { authService } from '$lib/services/auth-service';
	onMount(async () => {
		const query = new URLSearchParams(window.location.search);
		const code = query.get('code');
		const state = query.get('state');

		// Call auth service to exchange code for tokens
		const tokens = await authService.exchangeCodeForTokens(code, state);

		// Store tokens in localStorage
		localStorage.setItem('access_token', tokens.access_token);
		localStorage.setItem('refresh_token', tokens.refresh_token);

		// Redirect to dashboard
		window.location.href = '/app';
	});
</script>

<!-- A simple wait to be redirected page -->
<div class="flex items-center justify-center h-screen">
	<p>Wait to be redirected...</p>
</div>
