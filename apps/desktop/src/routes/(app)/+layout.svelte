<script lang="ts">
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}
	});
</script>

<Menubar />
<Sidebar.Provider open={false}>
	<MainSidebar />
	<main class="flex flex-col h-[calc(100vh-70px)] w-full">
		<div class="flex-1">{@render children?.()}</div>
	</main>
</Sidebar.Provider>
