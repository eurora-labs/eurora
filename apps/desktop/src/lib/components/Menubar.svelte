<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { onMount } from 'svelte';

	const taurpc = inject(TAURPC_SERVICE);

	onMount(() => {
		taurpc.auth
			.is_authenticated()
			.then((isAuthenticated) => {
				if (!isAuthenticated) {
					goto('/onboarding');
				}
			})
			.catch((error) => {
				goto('/onboarding');
				console.error('Failed to check authentication:', error);
			});
	});
</script>
