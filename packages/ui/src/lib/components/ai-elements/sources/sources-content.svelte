<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { CollapsibleContent } from '$lib/components/collapsible/index.js';

	let { class: className, ...restProps }: { class?: string; [key: string]: any } = $props();
</script>

<CollapsibleContent
	data-slot="sources-content"
	class={cn(
		'mt-3 flex w-fit flex-col gap-2',
		'data-[state=closed]:fade-out-0 data-[state=closed]:slide-out-to-top-2 data-[state=open]:slide-in-from-top-2 outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
		className,
	)}
	{...restProps}
/>
