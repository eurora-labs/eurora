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

<!--
 Each Timeline.Item renders one continuous vertical line with the icon
 stacked on top of its midpoint (via single-cell grid placement). The icon
 occludes the middle of the line, so adjacent items' lines abut to form a
 continuous track without any per-item visibility tricks at the Timeline
 level.
-->
