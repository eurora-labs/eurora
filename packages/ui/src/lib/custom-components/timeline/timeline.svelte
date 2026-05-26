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

<ol class={cn('flex flex-col items-center gap-1.5', className)} aria-label="Activity timeline">
	{@render children?.()}
</ol>
