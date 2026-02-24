<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<span
	data-slot="inline-citation-text"
	class={cn('transition-colors group-hover:bg-accent', className)}
	{...rest}
>
	{@render children?.()}
</span>
