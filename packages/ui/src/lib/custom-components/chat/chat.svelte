<script lang="ts" module>
	export interface ChatProps {
		class?: string;
		children?: any;
	}
</script>

<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	let { class: className, children }: ChatProps = $props();

	let scrollAreaRef = $state<HTMLDivElement>();
</script>

<ScrollArea ref={scrollAreaRef} class="w-full {className}">
	<div class="space-y-4 p-4 flex flex-col">
		{@render children?.()}
	</div>
</ScrollArea>
