<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { HoverCardContent } from '$lib/components/hover-card/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<HoverCardContent
	data-slot="context-usage-content"
	class={cn('min-w-60 divide-y overflow-hidden p-0', className)}
	{...rest}
>
	{@render children?.()}
</HoverCardContent>
