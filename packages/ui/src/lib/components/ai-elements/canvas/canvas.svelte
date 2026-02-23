<script lang="ts">
	import type { Snippet } from 'svelte';
	import { SvelteFlow, Background } from '@xyflow/svelte';
	import '@xyflow/svelte/dist/style.css';
	import { cn } from '$lib/utils.js';

	interface CanvasProps {
		class?: string;
		children?: Snippet;
		[key: string]: any;
	}

	let { class: className, children, ...restProps }: CanvasProps = $props();

	const deleteKey = ['Backspace', 'Delete'];
</script>

<div data-slot="canvas" class={cn('h-full w-full', className)}>
	<SvelteFlow
		{deleteKey}
		fitView
		panOnDrag={false}
		panOnScroll
		selectionOnDrag={true}
		zoomOnDoubleClick={false}
		{...restProps}
	>
		<Background bgColor="var(--sidebar)" />
		{@render children?.()}
	</SvelteFlow>
</div>
