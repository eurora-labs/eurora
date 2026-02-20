<script lang="ts">
	import '$lib/../app.css';
	import { Toaster } from '@eurora/ui/components/sonner/index';
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
<Toaster />
