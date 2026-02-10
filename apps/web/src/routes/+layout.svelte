<script lang="ts">
	import '../app.css';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	onMount(() => {
		setMode('dark');
	});

	let { children } = $props();
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="mx-0 w-full h-full">
	{@render children?.()}
</main>
