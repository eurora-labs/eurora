<script lang="ts">
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import { inject } from '@eurora/shared/context';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';

	let taurpcService = inject(TAURPC_SERVICE);
	let timelineItems: string[] = $state([]);

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}
		taurpcService.timeline.list().then((items) => {
			timelineItems = items;
		});
	});
</script>

<Menubar />
<Sidebar.Provider open={false}>
	<MainSidebar />
	<main class="flex flex-col h-[calc(100vh-70px)] w-full">
		<div class="flex-1">{@render children?.()}</div>

		<div class="flex">
			<Timeline.Root>
				{#each timelineItems as item}
					<Timeline.Item>{item}</Timeline.Item>
				{/each}
			</Timeline.Root>
		</div>
	</main>
</Sidebar.Provider>
