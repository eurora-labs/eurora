<script lang="ts">
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { onMount } from 'svelte';
	import { platform } from '@tauri-apps/plugin-os';

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();

	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);
	let scaleFactor = $state<number>(1.0);

	function resizeWindow() {
		if (!mainRef) return;
		try {
			// Use TauRPC resize_launcher_window
			taurpc.window.resize_launcher_window(500, 1.0);
		} catch (error) {
			console.error('Failed to resize window:', error);
		}
	}

	onMount(() => {
		document.body.classList.add(`${platform()}-app`);
		resizeWindow();
		// const resizeObserver = new ResizeObserver(resizeWindow);

		// Use TauRPC for get_scale_factor
		taurpc.window.get_scale_factor(mainRef?.scrollHeight || 100).then(async (result) => {
			scaleFactor = result;

			// resizeObserver.observe(mainRef!);
		});

		return () => {
			// resizeObserver.disconnect();
		};
	});
</script>

<main bind:this={mainRef} class="h-screen min-h-[100px] bg-transparent">
	{@render children?.()}
</main>
