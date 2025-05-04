<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className = '',
		href = '',
		target = '_self',
		disabled = false,
		children,
		...restProps
	} = $props();
</script>

<a
	class={cn(
		'aria-selected:bg-accent aria-selected:text-accent-foreground relative flex cursor-default select-none items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-none data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:pointer-events-none [&_svg]:size-4 [&_svg]:shrink-0',
		className
	)}
	bind:this={ref}
	{href}
	{target}
	data-disabled={disabled || undefined}
	tabindex={disabled ? -1 : 0}
	role="option"
	data-command-link-item
	{...restProps}
>
	{@render children?.()}
</a>
