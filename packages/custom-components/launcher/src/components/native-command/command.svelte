<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { cn } from '@eurora/ui/utils.js';

	let { value = $bindable(''), ref = $bindable(null), class: className = '', children } = $props();

	// Create event dispatcher for any events we need to forward
	const dispatch = createEventDispatcher();

	// Store for active item index and filtered items
	let activeIndex = -1;

	// Expose the setValue method to parent components
	function setValue(newValue: string) {
		value = newValue;
	}

	// Handle keydown for keyboard navigation
	function handleKeyDown(event: KeyboardEvent) {
		dispatch('keydown', event);
	}
</script>

<div
	class={cn(
		'text-popover-foreground flex h-full w-full flex-col overflow-hidden rounded-md border-none',
		className
	)}
	bind:this={ref}
	on:keydown={handleKeyDown}
	role="combobox"
	aria-haspopup="listbox"
	data-command-root
>
	{@render children?.()}
</div>
