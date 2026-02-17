<script lang="ts">
	import { goto } from '$app/navigation';
	import { type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let taurpcService = inject(TAURPC_SERVICE);
	let timelineItems: TimelineAppEvent[] = $state([]);
	let timelineOpen = $state(true);
	let roleChecked = $state(false);

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}

		taurpcService.auth
			.get_role()
			.then((role) => {
				if (role === 'Free') {
					goto('/onboarding/no-access');
				} else {
					roleChecked = true;
				}
			})
			.catch((error) => {
				console.error('Failed to check user role:', error);
				roleChecked = true;
			});

		taurpcService.timeline.new_app_event.on((e) => {
			if (timelineItems.length >= 2) {
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

{#if roleChecked}
	<Menubar />
	<Sidebar.Provider open={true}>
		<MainSidebar />
		<Sidebar.Inset>
			<div class="flex flex-col h-screen">
				<div class="flex-1 bg-background">{@render children?.()}</div>
				<div class="flex flex-col w-full">
					<Timeline.Root class="w-full" bind:open={timelineOpen} defaultOpen={false}>
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
			</div>
		</Sidebar.Inset>
	</Sidebar.Provider>
{:else}
	<div class="flex items-center justify-center h-screen">
		<Spinner class="size-8" />
	</div>
{/if}
