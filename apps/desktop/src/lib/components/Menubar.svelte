<script lang="ts">
	import { goto } from '$app/navigation';
	import { unwrap } from '$lib/bindings/result.js';
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { onMount } from 'svelte';

	onMount(() => {
		(async () => {
			try {
				const isAuthenticated = unwrap(await commands.authIsAuthenticated());
				if (!isAuthenticated) goto('/onboarding/login');
			} catch (error) {
				console.error('Failed to check authentication:', error);
				goto('/onboarding/login');
			}
		})();
	});
</script>
