<script lang="ts">
	import '$lib/../app.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { initAuthFromServer } from '$lib/stores/auth.js';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { data, children } = $props();

	initDependencies();
	initAuthFromServer(data.user ?? null);

	onMount(() => {
		setMode('dark');
	});
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="mx-0 w-full h-full">
	{@render children?.()}
</main>
<Toaster />
