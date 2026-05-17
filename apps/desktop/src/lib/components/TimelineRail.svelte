<script lang="ts">
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as ScrollArea from '@eurora/ui/components/scroll-area/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import { onMount } from 'svelte';

	const activityService = inject(ACTIVITY_SERVICE);

	// Tag <body> while the rail is mounted so peer chrome (sidebar, titlebar
	// leading region) can shift over to make room. A body class is more
	// portable across webviews than a `:has()` selector and survives the
	// Svelte component's own scope.
	onMount(() => {
		document.body.classList.add('has-timeline-rail');
		return () => document.body.classList.remove('has-timeline-rail');
	});

	// Connector height is mapped from activity duration on a log scale so the
	// rail stays legible across the realistic range (a few seconds of
	// task-switching up to multi-hour focus blocks). The anchors below define
	// the linear interpolation in log-space: REF_SHORT collapses to
	// MIN_HEIGHT, REF_LONG saturates at MAX_HEIGHT, anything between is
	// proportional. Tuning these is a pure UX decision — no other code
	// depends on the exact mapping.
	const MIN_CONNECTOR_HEIGHT_PX = 8;
	const MAX_CONNECTOR_HEIGHT_PX = 96;
	const REF_SHORT_SECONDS = 1;
	const REF_LONG_SECONDS = 30;

	const LOG_REF_SHORT = Math.log(REF_SHORT_SECONDS);
	const LOG_REF_LONG = Math.log(REF_LONG_SECONDS);

	function connectorHeightFor(startedAt: string, endedAt: string | null): number {
		// `endedAt` is null for the in-progress activity. That row is almost
		// always the first item (its connector is hidden by Timeline.Root's
		// :first-child rule), so a precise value here is cosmetic at best.
		// Falling back to MIN avoids needing a ticking clock for a
		// degenerate case.
		if (endedAt === null) return MIN_CONNECTOR_HEIGHT_PX;

		const durationSeconds = (Date.parse(endedAt) - Date.parse(startedAt)) / 1000;
		if (!Number.isFinite(durationSeconds) || durationSeconds <= REF_SHORT_SECONDS) {
			return MIN_CONNECTOR_HEIGHT_PX;
		}
		if (durationSeconds >= REF_LONG_SECONDS) return MAX_CONNECTOR_HEIGHT_PX;

		const t = (Math.log(durationSeconds) - LOG_REF_SHORT) / (LOG_REF_LONG - LOG_REF_SHORT);
		return MIN_CONNECTOR_HEIGHT_PX + t * (MAX_CONNECTOR_HEIGHT_PX - MIN_CONNECTOR_HEIGHT_PX);
	}
</script>

<aside
	data-slot="timeline-rail"
	class="bg-sidebar relative z-30 flex h-full w-12 shrink-0 flex-col"
	aria-label="Recent activity"
>
	<ScrollArea.Root class="min-h-0 flex-1" scrollbarYClasses="w-1.5">
		<Timeline.Root>
			{#each activityService.recent as item, i (item.id)}
				<Timeline.Item
					color={item.accent?.hex}
					highlighted={i === 0}
					iconSrc={item.iconBase64}
					name={item.name}
					connectorHeight={connectorHeightFor(item.startedAt, item.endedAt)}
				/>
			{/each}
		</Timeline.Root>
	</ScrollArea.Root>
</aside>
