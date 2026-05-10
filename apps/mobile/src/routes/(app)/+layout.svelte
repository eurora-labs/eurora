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

	// DEBUG: nightly-only sidebar freeze. Remove once resolved.
	let debugTick = $state(0);
	let debugBodyPe = $state('-');
	let debugHtmlPe = $state('-');
	let debugBodyOf = $state('-');
	let debugLayers = $state(-1);
	onMount(() => {
		const id = window.setInterval(() => {
			debugTick = (debugTick + 1) % 1000;
			debugBodyPe = document.body.style.pointerEvents || '-';
			debugHtmlPe = document.documentElement.style.pointerEvents || '-';
			debugBodyOf = document.body.style.overflow || '-';
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			debugLayers = (globalThis as any).bitsDismissableLayers?.size ?? -1;
		}, 200);
		return () => window.clearInterval(id);
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

<!-- DEBUG: nightly-only sidebar freeze. Remove once resolved. -->
<div
	style="position:fixed;top:env(safe-area-inset-top);right:0;z-index:99999;background:red;color:white;font-family:monospace;font-size:10px;padding:2px 4px;pointer-events:none;line-height:1.2;"
>
	t{debugTick} bpe:{debugBodyPe} hpe:{debugHtmlPe} of:{debugBodyOf} l:{debugLayers}
</div>
