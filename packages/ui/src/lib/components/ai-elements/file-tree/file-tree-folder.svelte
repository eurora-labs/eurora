<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Collapsible, CollapsibleTrigger, CollapsibleContent } from '$lib/components/collapsible/index.js';
	import ChevronRight from '@lucide/svelte/icons/chevron-right';
	import Folder from '@lucide/svelte/icons/folder';
	import FolderOpen from '@lucide/svelte/icons/folder-open';
	import { useFileTree } from './file-tree-context.svelte.js';

	let {
		class: className,
		path,
		name,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		path: string;
		name: string;
		children?: Snippet;
	} = $props();

	const ctx = useFileTree();

	let isExpanded = $derived(ctx.expandedPaths.has(path));
	let isSelected = $derived(ctx.selectedPath === path);

	function handleToggle() {
		ctx.togglePath(path);
	}

	function handleSelect() {
		ctx.select(path);
	}
</script>

<Collapsible open={isExpanded} onOpenChange={handleToggle}>
	<div
		data-slot="file-tree-folder"
		class={cn('', className)}
		role="treeitem"
		tabindex={0}
		{...restProps}
	>
		<CollapsibleTrigger
			class={cn(
				'flex w-full items-center gap-1 rounded px-2 py-1 text-left transition-colors hover:bg-muted/50',
				isSelected && 'bg-muted',
			)}
			onclick={handleSelect}
		>
			<ChevronRight
				class={cn(
					'size-4 shrink-0 text-muted-foreground transition-transform',
					isExpanded && 'rotate-90',
				)}
			/>
			<span class="shrink-0">
				{#if isExpanded}
					<FolderOpen class="size-4 text-blue-500" />
				{:else}
					<Folder class="size-4 text-blue-500" />
				{/if}
			</span>
			<span class="truncate">{name}</span>
		</CollapsibleTrigger>
		<CollapsibleContent>
			<div class="ml-4 border-l pl-2">
				{@render children?.()}
			</div>
		</CollapsibleContent>
	</div>
</Collapsible>
