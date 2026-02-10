<script lang="ts">
	import '../app.css';
	// import * as Alert from '@eurora/ui/components/alert/index';
	// import { Button } from '@eurora/ui/components/button/index';
	// import XIcon from '@lucide/svelte/icons/x';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	onMount(() => {
		setMode('dark');
	});

	let { children } = $props();
	// let showAlert = $state(true);
</script>

<ModeWatcher defaultMode="dark" track={false} />

<main class="mx-0 w-full h-full">
	{@render children?.()}
</main>
