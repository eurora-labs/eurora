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

<blockquote
	data-slot="inline-citation-quote"
	class={cn('border-muted text-muted-foreground border-l-2 pl-3 text-sm italic', className)}
	{...rest}
>
	{@render children?.()}
</blockquote>
