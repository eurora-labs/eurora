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
		// taurpcService.timeline.list().then((items) => {
		// 	timelineItems = items;
		// });

		taurpcService.timeline.new_app_event.on((e) => {
			timelineItems.push(e.name);
		});
	});
</script>

<Menubar />
<Sidebar.Provider open={false}>
	<MainSidebar />
	<Sidebar.Inset>
		<div class="flex flex-col h-[calc(100vh-70px)]">
			<div class="flex-1">{@render children?.()}</div>
			<div class="flex mb-4">
				<Timeline.Root class="w-1 flex-1">
					{#each timelineItems as item}
						<Timeline.Item>{item}</Timeline.Item>
					{/each}
				</Timeline.Root>
			</div>
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
