<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.pcss';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';
	import { initDependencies } from '$lib/bootstrap/deps';

	let { children } = $props();

	initDependencies();

	onMount(() => {
		setMode('dark');
	});
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="p-0 m-0 bg-inherit">
	{@render children?.()}
</main>
