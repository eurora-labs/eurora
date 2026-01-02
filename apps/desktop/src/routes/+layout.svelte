<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { children } = $props();

	initDependencies();

	onMount(() => {
		setMode('dark');
	});
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="p-0 m-0 bg-inherit h-screen">
	{@render children?.()}
</main>
