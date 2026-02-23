<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import FileIcon from '@lucide/svelte/icons/file';
	import { useFileTree } from './file-tree-context.svelte.js';

	let {
		class: className,
		path,
		name,
		icon,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		path: string;
		name: string;
		icon?: Snippet;
		children?: Snippet;
	} = $props();

	const ctx = useFileTree();

	let isSelected = $derived(ctx.selectedPath === path);

	function handleClick() {
		ctx.select(path);
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' || e.key === ' ') {
			ctx.select(path);
		}
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	data-slot="file-tree-file"
	class={cn(
		'flex cursor-pointer items-center gap-1 rounded px-2 py-1 transition-colors hover:bg-muted/50',
		isSelected && 'bg-muted',
		className,
	)}
	onclick={handleClick}
	onkeydown={handleKeyDown}
	role="treeitem"
	tabindex={0}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		<span class="size-4"></span>
		<span class="shrink-0">
			{#if icon}
				{@render icon()}
			{:else}
				<FileIcon class="size-4 text-muted-foreground" />
			{/if}
		</span>
		<span class="truncate">{name}</span>
	{/if}
</div>
