<script lang="ts">
	import { createTauRPCProxy } from '@eurora/tauri-bindings';
	import { onMount } from 'svelte';

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();

	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);
	let scaleFactor = $state<number>(1.0);

	function resizeWindow() {
		if (!mainRef) return;
		try {
			// Use TauRPC resize_launcher_window
			taurpc.window.resize_launcher_window(mainRef.scrollHeight, scaleFactor);
		} catch (error) {
			console.error('Failed to resize window:', error);
		}
	}

	onMount(() => {
		const resizeObserver = new ResizeObserver(resizeWindow);

		// Use TauRPC for get_scale_factor
		taurpc.window.get_scale_factor(mainRef?.scrollHeight || 100).then(async (result) => {
			scaleFactor = result;

			resizeObserver.observe(mainRef!);
		});

		return () => {
			resizeObserver.disconnect();
		};
	});
</script>

<main bind:this={mainRef} class="h-fit min-h-[100px] flex-1 bg-transparent">
	{@render children?.()}
</main>
