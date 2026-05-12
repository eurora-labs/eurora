<script lang="ts">
	import EuroraIcon from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		document.body.classList.add(`${platform()}-app`);
	});
</script>

<div class="flex flex-row h-full w-full">
	<div class="w-1/2">{@render children?.()}</div>
	<div class="flex flex-col gap-4 w-1/2 justify-center items-center">
		<EuroraIcon size="128" />
		<p>Designed in The Netherlands</p>
	</div>
</div>
