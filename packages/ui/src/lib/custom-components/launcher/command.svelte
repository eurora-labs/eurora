<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';

	let {
		value = $bindable(''),
		ref = $bindable(null),
		class: className,
		...restProps
	}: CommandPrimitive.RootProps = $props();
</script>

<CommandPrimitive.Root
	class={cn(
		'text-popover-foreground flex h-full w-full flex-col overflow-hidden rounded-md border-none',
		className,
	)}
	bind:ref
	bind:value
	{...restProps}
></CommandPrimitive.Root>
