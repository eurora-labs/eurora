<script lang="ts">
	import { events, type TimelineAppEvent } from '$lib/bindings/specta.bindings.js';
	import EuroraIcon from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let timelineItems: TimelineAppEvent[] = $state([]);
	let newAppEventListener: (() => void) | undefined;

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}

		events.timelineAppEvent
			.listen((e) => {
				if (timelineItems.length >= 5) {
					timelineItems.shift();
				}
				timelineItems.push(e.payload);
			})
			.then((listener) => {
				newAppEventListener = listener;
			});

		return () => {
			newAppEventListener?.();
		};
	});
</script>

<div class="flex flex-row h-full">
	<div class="w-1/2">{@render children?.()}</div>
	<div class="flex flex-col gap-4 w-1/2 justify-center items-center">
		<EuroraIcon size="128" />
		<p>Designed in The Netherlands</p>
	</div>
</div>
