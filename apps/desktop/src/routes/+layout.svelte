<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.pcss';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';
	let { children } = $props();
	onMount(() => {
		setMode('light');
	});
</script>

<ModeWatcher defaultMode="light" track={false} />

<main class="h-screen p-0 m-0">
	{@render children?.()}
</main>
