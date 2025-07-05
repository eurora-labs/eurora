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
	let bottomRef = $state<HTMLDivElement>();

	export function scrollToBottom() {
		bottomRef?.scrollIntoView({ behavior: 'smooth', block: 'end' });
	}
</script>

<ScrollArea ref={scrollAreaRef} class="w-full {className}">
	<div class="space-y-4 p-4 pb-0 flex flex-col">
		{@render children?.()}
	</div>
	<div class="h-4 w-0" bind:this={bottomRef}></div>
</ScrollArea>
