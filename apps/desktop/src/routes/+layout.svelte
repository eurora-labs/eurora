<script lang="ts">
	import '@eurora/ui/main.css';
	import '../app.css';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';
	let { children } = $props();
	onMount(() => {
		document.body.classList.add(`${platform()}-app`);
	});
</script>

{@render children?.()}
