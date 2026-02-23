<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import BookmarkIcon from '@lucide/svelte/icons/bookmark';

	let {
		class: className,
		children,
		...restProps
	}: { class?: string; children?: Snippet; [key: string]: any } = $props();
</script>

{#if children}
	{@render children()}
{:else}
	<BookmarkIcon data-slot="checkpoint-icon" class={cn('size-4 shrink-0', className)} {...restProps} />
{/if}
