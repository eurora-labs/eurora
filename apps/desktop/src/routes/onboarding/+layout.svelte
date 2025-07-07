<script lang="ts">
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';
	import Menubar from './Menubar.svelte';
	let { children } = $props();
	onMount(() => {
		document.body.classList.add(`${platform()}-app`);
	});
</script>

<div class="flex flex-col h-screen">
	<Menubar />
	<div class="flex-1">{@render children?.()}</div>
</div>
