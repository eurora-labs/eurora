<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { setFileTree } from './file-tree-context.svelte.js';

	let {
		class: className,
		expanded: controlledExpanded,
		defaultExpanded = new Set<string>(),
		selectedPath,
		onSelect,
		onExpandedChange,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		expanded?: Set<string>;
		defaultExpanded?: Set<string>;
		selectedPath?: string;
		onSelect?: (path: string) => void;
		onExpandedChange?: (expanded: Set<string>) => void;
		children?: Snippet;
	} = $props();

	let internalExpanded = $state(defaultExpanded);
	let expandedPaths = $derived(controlledExpanded ?? internalExpanded);

	setFileTree({
		expandedPaths: () => expandedPaths,
		togglePath: (path: string) => {
			const newExpanded = new Set(expandedPaths);
			if (newExpanded.has(path)) {
				newExpanded.delete(path);
			} else {
				newExpanded.add(path);
			}
			internalExpanded = newExpanded;
			onExpandedChange?.(newExpanded);
		},
		selectedPath: () => selectedPath,
		onSelect: () => onSelect,
	});
</script>

<div
	data-slot="file-tree"
	class={cn('rounded-lg border bg-background font-mono text-sm', className)}
	role="tree"
	{...restProps}
>
	<div class="p-2">
		{@render children?.()}
	</div>
</div>
