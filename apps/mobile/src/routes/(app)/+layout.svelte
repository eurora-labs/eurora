<script lang="ts">
	import { goto } from '$app/navigation';
	import MobileSidebar from '$lib/components/MobileSidebar.svelte';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { onMount } from 'svelte';

	let { children } = $props();

	const user = inject(USER_SERVICE);
	const chatService = inject(CHAT_SERVICE);

	$effect(() => {
		if (user.initialized && !user.authenticated) {
			goto('/login');
		}
	});

	onMount(() => {
		return () => {
			chatService.destroy();
		};
	});
</script>

{#if !user.initialized}
	<div class="flex h-dvh items-center justify-center">
		<Spinner class="w-8 h-8" />
	</div>
{:else if user.authenticated}
	<Sidebar.Provider class="h-dvh min-h-dvh">
		<MobileSidebar />
		<Sidebar.Inset class="h-dvh min-h-0">
			<header class="flex shrink-0 items-center gap-2 border-b border-border px-3 py-2">
				<Sidebar.Trigger />
				<h1 class="text-sm font-semibold text-foreground">Eurora</h1>
			</header>
			{@render children?.()}
		</Sidebar.Inset>
	</Sidebar.Provider>
{/if}
