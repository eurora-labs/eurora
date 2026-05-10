<script lang="ts">
	import '$lib/../app.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { warmupShikiHighlighter } from '@eurora/ui/components/ai-elements/message/shiki/index';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { children } = $props();

	initDependencies();

	// Boot the syntax-highlighter worker and pre-load common languages so
	// the first streamed code block doesn't pay grammar-load latency.
	warmupShikiHighlighter();

	onMount(() => {
		setMode('dark');
	});
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="mx-0 w-full h-full">
	{@render children?.()}
</main>
<Toaster />
