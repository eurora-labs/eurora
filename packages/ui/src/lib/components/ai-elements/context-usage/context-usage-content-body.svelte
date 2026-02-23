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

<div data-slot="context-usage-content-body" class={cn('w-full p-3', className)} {...rest}>
	{@render children?.()}
</div>
