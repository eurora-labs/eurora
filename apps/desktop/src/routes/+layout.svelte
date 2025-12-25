<script lang="ts">
	import '../app.css';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';
	import { initDependencies } from '$lib/bootstrap/deps.js';

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
