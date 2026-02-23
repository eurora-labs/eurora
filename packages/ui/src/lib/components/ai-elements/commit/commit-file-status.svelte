<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	type FileStatus = 'added' | 'modified' | 'deleted' | 'renamed';

	const fileStatusStyles: Record<FileStatus, string> = {
		added: 'text-green-600 dark:text-green-400',
		deleted: 'text-red-600 dark:text-red-400',
		modified: 'text-yellow-600 dark:text-yellow-400',
		renamed: 'text-blue-600 dark:text-blue-400',
	};

	const fileStatusLabels: Record<FileStatus, string> = {
		added: 'A',
		deleted: 'D',
		modified: 'M',
		renamed: 'R',
	};

	let {
		class: className,
		status,
		children,
		...restProps
	}: HTMLAttributes<HTMLSpanElement> & {
		status: FileStatus;
		children?: Snippet;
	} = $props();
</script>

<span
	data-slot="commit-file-status"
	class={cn('font-medium font-mono text-xs', fileStatusStyles[status], className)}
	{...restProps}
>
	{#if children}
		{@render children()}
	{:else}
		{fileStatusLabels[status]}
	{/if}
</span>
