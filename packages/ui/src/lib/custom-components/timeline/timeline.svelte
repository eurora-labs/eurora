<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils.js';

	let { class: className, children }: TimelineProps = $props();
</script>

<ol class={cn('flex flex-col items-center', className)} aria-label="Activity timeline">
	{@render children?.()}
</ol>

<style>
	/*
	 * Each Timeline.Item draws a short vertical connector above its icon,
	 * tinted with the item's accent color, so the column reads as a single
	 * track. The first item (newest, at the top) has no upstream item, so
	 * its connector collapses entirely — letting the newest icon sit flush
	 * with the top of the container instead of leaving a leading gap.
	 */
	ol > :global(li:first-child > .timeline-connector) {
		display: none;
	}
</style>
