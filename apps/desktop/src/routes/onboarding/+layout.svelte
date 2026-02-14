<script lang="ts">
	import { type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import EuroraIcon from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let taurpcService = inject(TAURPC_SERVICE);
	let timelineItems: TimelineAppEvent[] = $state([]);
	let newAppEventListener: () => void;

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}

		taurpcService.timeline.new_app_event
			.on((e) => {
				// Limit the items to 5
				if (timelineItems.length >= 5) {
					timelineItems.shift();
				}
				timelineItems.push(e);
			})
			.then((listener) => {
				newAppEventListener = listener;
			});

		return () => {
			newAppEventListener?.();
		};
	});
</script>

<div class="flex flex-row h-screen">
	<div class="w-1/2">{@render children?.()}</div>
	<div class="flex flex-col gap-4 w-1/2 justify-center items-center">
		<EuroraIcon size="128" />
		<p>Designed in The Netherlands</p>
	</div>
</div>
