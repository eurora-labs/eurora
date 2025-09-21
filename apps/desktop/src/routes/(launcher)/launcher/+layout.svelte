<script lang="ts">
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { scaleFactor } from './scaleFactor.svelte.js';
	import { onMount } from 'svelte';
	import { platform } from '@tauri-apps/plugin-os';

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();

	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);

	function resizeWindow() {
		if (!mainRef) return;
		try {
			// Use TauRPC resize_launcher_window
			taurpc.window.resize_launcher_window(100, scaleFactor.value);
		} catch (error) {
			console.error('Failed to resize window:', error);
		}
	}

	taurpc.window.launcher_opened.on(async (info) => {
		console.log(
			'scroll height:',
			mainRef?.clientHeight,
			mainRef?.scrollHeight,
			mainRef?.offsetHeight,
		);
		taurpc.window.get_scale_factor(mainRef?.scrollHeight || 100).then(async (result) => {
			scaleFactor.value = result;
			console.log('Scale factor:', scaleFactor);
			taurpc.window.resize_launcher_window(100, scaleFactor.value).then(() => {
				console.log('Window resized');
			});

			// resizeObserver.observe(mainRef!);
		});
	});

	onMount(() => {
		document.body.classList.add(`${platform()}-app`);

		// resizeWindow();
		// const resizeObserver = new ResizeObserver(resizeWindow);

		// Use TauRPC for get_scale_factor
		// taurpc.window.get_scale_factor(mainRef?.scrollHeight || 100).then(async (result) => {
		// 	scaleFactor = result;
		// 	console.log('Scale factor:', scaleFactor);
		// 	taurpc.window.resize_launcher_window(mainRef?.scrollHeight || 100, scaleFactor);

		// 	// resizeObserver.observe(mainRef!);
		// });

		return () => {
			// resizeObserver.disconnect();
		};
	});
</script>

<main bind:this={mainRef} class="h-screen bg-transparent">
	{@render children?.()}
</main>
