<script lang="ts">
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { inject } from '@eurora/shared/context';
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

	// Minimum gap between wheel-driven cycles. Each non-zero wheel event
	// fires exactly one cycle in the direction of its `deltaY`, but we
	// rate-limit to keep trackpad inertia and fast wheel echoes from
	// runaway-cycling past the user's intended target. 80ms tolerates
	// deliberate wheel spinning (~12 notches/sec) while still throttling
	// kinetic-scroll bursts.
	const WHEEL_COOLDOWN_MS = 80;

	function connectorHeightFor(startedAt: string, endedAt: string | null): number {
		// `endedAt: null` uniquely identifies the in-progress activity:
		// once a row stops being current, the `savedActivityEnded` event
		// patches its `endedAt` in place (see ActivityService.applyEnded).
		// The in-progress row's connector is hidden by Timeline.Root's
		// :first-child rule, so a precise value here is cosmetic at best
		// — falling back to MIN avoids needing a ticking clock for it.
		if (endedAt === null) return MIN_CONNECTOR_HEIGHT_PX;

		const durationSeconds = (Date.parse(endedAt) - Date.parse(startedAt)) / 1000;
		if (!Number.isFinite(durationSeconds) || durationSeconds <= REF_SHORT_SECONDS) {
			return MIN_CONNECTOR_HEIGHT_PX;
		}
		if (durationSeconds >= REF_LONG_SECONDS) return MAX_CONNECTOR_HEIGHT_PX;

		const t = (Math.log(durationSeconds) - LOG_REF_SHORT) / (LOG_REF_LONG - LOG_REF_SHORT);
		return MIN_CONNECTOR_HEIGHT_PX + t * (MAX_CONNECTOR_HEIGHT_PX - MIN_CONNECTOR_HEIGHT_PX);
	}

	// Rotate the chronological list so the active item is always the
	// top-most rendered row. The relative chronology between items is
	// preserved within the cycle — connector heights and visual identity
	// of each item are unchanged by rotation. Keyed by activity id so
	// Svelte transitions the rearrangement instead of recreating DOM.
	const display = $derived.by(() => {
		const r = activityService.recent;
		if (r.length === 0) return [] as typeof r;
		const i = activityService.activeIndex;
		return [...r.slice(i), ...r.slice(0, i)];
	});

	// Wheel handling — one cycle per event, gated by a cooldown. The
	// accumulator-with-threshold pattern we tried first was too sensitive
	// to per-platform `deltaY` magnitudes (Linux X11 ~53px, Windows ~120px,
	// macOS ~100px), leaving some platforms stuck below threshold per
	// notch. This handles only the *sign* of `deltaY`, so the behaviour is
	// identical regardless of how loud the platform reports the gesture.
	// `markScrolling` inside the service still owns the scrolling=false
	// flip after the user goes idle.
	let lastWheelCycleAt = 0;

	function onWheel(event: WheelEvent): void {
		event.preventDefault();
		if (event.deltaY === 0) return;

		const now = performance.now();
		if (now - lastWheelCycleAt < WHEEL_COOLDOWN_MS) return;

		const moved =
			event.deltaY > 0 ? activityService.cycleNext() : activityService.cyclePrevious();
		// Only charge the cooldown for movements that actually happened.
		// Hammering scroll-up at index 0 should not later block a deliberate
		// scroll-down by 80ms of stale throttle.
		if (moved) lastWheelCycleAt = now;
	}

	function onKeydown(event: KeyboardEvent): void {
		switch (event.key) {
			case 'ArrowDown':
			case 'PageDown':
				event.preventDefault();
				activityService.cycleNext();
				break;
			case 'ArrowUp':
			case 'PageUp':
				event.preventDefault();
				activityService.cyclePrevious();
				break;
			default:
				break;
		}
	}
</script>

<aside
	data-slot="timeline-rail"
	class="bg-sidebar relative z-30 flex h-full w-12 shrink-0 flex-col overflow-hidden"
	aria-label="Recent activity"
>
	<div
		class="contents focus:outline-none"
		role="listbox"
		tabindex="0"
		aria-label="Recent activity"
		aria-activedescendant={activityService.activeApp
			? `timeline-rail-item-${activityService.activeApp.id}`
			: undefined}
		onwheel={onWheel}
		onkeydown={onKeydown}
	>
		<Timeline.Root>
			{#each display as item, i (item.id)}
				<Timeline.Item
					id={`timeline-rail-item-${item.id}`}
					color={item.accent?.hex}
					active={i === 0}
					iconSrc={item.iconBase64}
					name={item.name}
					connectorHeight={connectorHeightFor(item.startedAt, item.endedAt)}
				/>
			{/each}
		</Timeline.Root>
	</div>
</aside>
