<script lang="ts">
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';
	import Menubar from '$lib/components/Menubar.svelte';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import * as Collapsible from '@eurora/ui/components/collapsible/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import { inject } from '@eurora/shared/context';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { type TimelineAppEvent } from '$lib/bindings/bindings.js';

	let taurpcService = inject(TAURPC_SERVICE);
	let timelineItems: TimelineAppEvent[] = $state([]);
	let timelineOpen = $state(true);

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

	function getFirstLetterAndCapitalize(name: string) {
		if (!name) return '';
		return name.charAt(0).toUpperCase();
	}
</script>

<Menubar />
<Sidebar.Provider open={false}>
	<MainSidebar />
	<Sidebar.Inset>
		<div class="flex flex-col h-[calc(100vh-70px)]">
			<div class="flex-1">{@render children?.()}</div>
			<Collapsible.Root bind:open={timelineOpen} class="w-full">
				<Collapsible.Trigger class="w-full">
					<div class="flex flex-row w-full justify-end items-end">
						{#if timelineOpen}
							<ChevronDown />
						{:else}
							<ChevronUp />
						{/if}
					</div>
				</Collapsible.Trigger>
				<Collapsible.Content>
					<div class="flex flex-col">
						<div class="flex w-full justify-center mb-4">Now</div>
						<div class="flex flex-row mb-4 h-[100px]">
							<div class="flex w-1/2 items-center">
								<Timeline.Root class="w-1 flex-1 h-fit" align="right">
									{#each timelineItems as item}
										<Timeline.Item color={item.color}>
											{#if item.icon_base64}<img
													src={item.icon_base64}
													alt={item.name}
													class="w-8 h-8 bg-white rounded-full drop-shadow p-1"
												/>{:else}
												<div
													class="w-8 h-8 bg-white rounded-full drop-shadow p-1 flex items-center justify-center"
												>
													{getFirstLetterAndCapitalize(item.name)}
												</div>
											{/if}
										</Timeline.Item>
									{/each}
								</Timeline.Root>
							</div>
							<div class="flex w-[200px] gap-2 h-full">
								<div class="h-full w-[5px] border rounded-full"></div>
							</div>
						</div>
					</div>
				</Collapsible.Content>
			</Collapsible.Root>
		</div>
	</Sidebar.Inset>
</Sidebar.Provider>
