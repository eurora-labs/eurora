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

<span data-slot="inline-citation" class={cn('group inline items-center gap-1', className)} {...rest}>
	{@render children?.()}
</span>
