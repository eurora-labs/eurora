<script lang="ts">
	import { type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let taurpcService = inject(TAURPC_SERVICE);
	let timelineItems: TimelineAppEvent[] = $state([]);

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}
		// taurpcService.timeline.list().then((items) => {
		// 	timelineItems = items;
		// });

		taurpcService.timeline.new_app_event.on((e) => {
			// Limit the items to 5
			if (timelineItems.length >= 5) {
				timelineItems.shift();
			}
			timelineItems.push(e);
		});
	});
</script>

<Sidebar.Provider open={false}>
	<Sidebar.Inset>
		<div class="flex flex-col h-screen">
			<div class="flex-1">{@render children?.()}</div>
			<div class="flex flex-col w-full"></div>
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
