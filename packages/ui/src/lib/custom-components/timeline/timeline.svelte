<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
	}
</script>

<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	let { class: className, children }: TimelineProps = $props();

	let scrollAreaRef = $state<HTMLDivElement>();
</script>

<ScrollArea ref={scrollAreaRef} orientation="horizontal" class="w-1/2 {className}">
	<div class="flex flex-row gap-2">
		{@render children?.()}
	</div>
</ScrollArea>
