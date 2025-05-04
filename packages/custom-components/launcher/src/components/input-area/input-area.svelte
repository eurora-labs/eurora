<script lang="ts">
	import type { WithElementRef, WithoutChildren } from 'bits-ui';
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable<HTMLTextAreaElement | null>(null),
		value = $bindable(''),
		class: className,
		...restProps
	}: WithoutChildren<WithElementRef<HTMLTextareaAttributes>> = $props();

	// Calculate and set the textarea height
	function adjustHeight(event: Event) {
		const target = event.target as HTMLTextAreaElement;
		if (!target) return;
		target.style.height = `${target.scrollHeight}px`; // Set to scroll height
	}
</script>

<textarea
	bind:this={ref}
	bind:value
	class={cn(
		'border-input focus-visible:ring-ring flex w-full resize-none overflow-hidden rounded-md border bg-transparent px-3 py-2 shadow-sm focus-visible:outline-none focus-visible:ring-1 disabled:cursor-not-allowed disabled:opacity-50',
		className
	)}
	oninput={adjustHeight}
	{...restProps}
></textarea>

<style>
	textarea::placeholder {
		color: rgba(0, 0, 0, 0.25);
		text-align: start;
	}
</style>
