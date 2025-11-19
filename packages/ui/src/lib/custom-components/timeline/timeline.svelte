<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
		align?: 'left' | 'right';
	}
</script>

<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	let { class: className, children, align = 'left' }: TimelineProps = $props();

	let scrollAreaRef = $state<HTMLDivElement>();

	const justifyClass = $derived(align === 'right' ? 'justify-end' : 'justify-start');
</script>

<ScrollArea
	ref={scrollAreaRef}
	orientation="horizontal"
	class="w-full whitespace-nowrap {className}"
>
	<div class="flex w-full {justifyClass}">
		<div class="flex flex-row w-max gap-2">
			{@render children?.()}
		</div>
	</div>
</ScrollArea>
