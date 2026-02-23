<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import GitCommit from '@lucide/svelte/icons/git-commit';

	let {
		class: className,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		children?: Snippet;
	} = $props();
</script>

<span data-slot="commit-hash" class={cn('font-mono text-xs', className)} {...restProps}>
	<GitCommit class="mr-1 inline-block size-3" />
	{@render children?.()}
</span>
