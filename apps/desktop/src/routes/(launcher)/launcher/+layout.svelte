<script lang="ts">
	import { onMount } from 'svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { inject } from '@eurora/shared/context';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';

	const taurpc = inject(TAURPC_SERVICE);

	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);

	taurpc.window.launcher_opened.on(async () => {
		console.log(
			'scroll height:',
			mainRef?.clientHeight,
			mainRef?.scrollHeight,
			mainRef?.offsetHeight,
		);
		// resizeLauncherWindow();
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

<main bind:this={mainRef} class="h-full bg-transparent">
	{@render children?.()}
</main>
