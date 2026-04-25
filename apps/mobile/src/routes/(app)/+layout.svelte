<script lang="ts">
	import { goto } from '$app/navigation';
	import MobileSidebar from '$lib/components/MobileSidebar.svelte';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { onMount } from 'svelte';

	let { children } = $props();

	const user = inject(USER_SERVICE);
	let ready = $state(false);

	onMount(() => {
		if (!user.authenticated) {
			goto('/login');
			return;
		}
		ready = true;
	});

	$effect(() => {
		if (ready && !user.authenticated) {
			goto('/login');
		}
	});
</script>

{#if ready}
	<Sidebar.Provider class="h-dvh min-h-dvh">
		<MobileSidebar />
		<Sidebar.Inset>
			<header class="flex items-center gap-2 px-3 py-2 border-b border-border">
				<Sidebar.Trigger />
				<h1 class="text-sm font-semibold text-foreground">Eurora</h1>
			</header>
			<main class="flex-1 min-h-0 bg-background">
				{@render children?.()}
			</main>
		</Sidebar.Inset>
	</Sidebar.Provider>
{/if}
