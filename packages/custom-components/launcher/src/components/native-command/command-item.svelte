<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className = '',
		disabled = false,
		value = '',
		children,
		select,
		keydown,
		...restProps
	} = $props();

	// Track if item is selected/highlighted
	let selected = false;

	// Handle click events
	function handleClick(event: MouseEvent) {
		if (!disabled) {
			select({ value });
		}
	}

	// Handle keydown for keyboard navigation/selection
	function handleKeyDown(event: KeyboardEvent) {
		if (disabled) return;

		if (event.key === 'Enter' || event.key === ' ') {
			event.preventDefault();
			select({ value });
		}

		keydown(event);
	}
</script>

<div
	class={cn(
		'aria-selected:bg-accent aria-selected:text-accent-foreground relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
		className
	)}
	bind:this={ref}
	role="option"
	aria-selected={selected}
	data-disabled={disabled || undefined}
	tabindex={disabled ? -1 : 0}
	on:click={handleClick}
	on:keydown={handleKeyDown}
	data-command-item
	{...restProps}
>
	{@render children?.()}
</div>
