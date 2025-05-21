<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import '../../app.pcss';
	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);
	let scaleFactor = $state<number>(1.0);

	function resizeWindow() {
		if (!mainRef) return;
		try {
			invoke('resize_window', {
				height: mainRef.scrollHeight,
				scaleFactor: scaleFactor
			});
		} catch (error) {
			console.error('Failed to resize window:', error);
		}
	}

	onMount(() => {
		const resizeObserver = new ResizeObserver(resizeWindow);

		invoke('get_scale_factor', { height: mainRef?.scrollHeight }).then(async (result) => {
			scaleFactor = result as number;

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
