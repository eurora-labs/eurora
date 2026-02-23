<script lang="ts">
	import * as HoverCard from '$lib/components/hover-card/index.js';
	import { cn } from '$lib/utils.js';

	interface Props {
		class?: string;
		align?: 'start' | 'center' | 'end';
		children?: import('svelte').Snippet;
	}

	let { class: className, align = 'start', children, ...restProps }: Props = $props();
</script>

<HoverCard.Content
	data-slot="attachment-hover-card-content"
	{align}
	class={cn('w-auto p-2', className)}
	{...restProps}
>
	{@render children?.()}
</HoverCard.Content>
