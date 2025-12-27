<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { children } = $props();

	initDependencies();

	onMount(() => {
		setMode('light');
	});
</script>

<ModeWatcher defaultMode="light" track={false} />

<main class="p-0 m-0 bg-inherit">
	{@render children?.()}
</main>
