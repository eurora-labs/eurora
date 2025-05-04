<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let {
		value = $bindable(''),
		ref = $bindable(null),
		class: className = '',
		children,
		keydown = $bindable(() => {})
	} = $props();

	// Store for active item index and filtered items
	let activeIndex = -1;

	// Expose the setValue method to parent components
	function setValue(newValue: string) {
		value = newValue;
	}

	// Handle keydown for keyboard navigation
	function handleKeyDown(event: KeyboardEvent) {
		keydown(event);
	}
</script>

<div
	class={cn(
		'text-popover-foreground flex h-full w-full flex-col overflow-hidden rounded-md border-none',
		className
	)}
	tabindex="0"
	bind:this={ref}
	onkeydown={handleKeyDown}
	role="combobox"
	aria-controls="listbox"
	aria-expanded="false"
	aria-haspopup="listbox"
	data-command-root
>
	{@render children?.()}
</div>
