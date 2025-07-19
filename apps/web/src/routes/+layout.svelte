<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.pcss';
	import { onMount } from 'svelte';
	import { ModeWatcher, setMode } from 'mode-watcher';

	onMount(() => {
		setMode('dark');
	});

	let { children } = $props();
</script>

<ModeWatcher defaultMode="dark" track={false} />

<div class="mx-0 w-full h-full">
	{@render children?.()}
</div>
