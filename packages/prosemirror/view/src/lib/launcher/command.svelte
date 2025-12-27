<script lang="ts">
	import { cn } from '$lib/utils';
	import { Command as CommandPrimitive } from 'bits-ui';

	let {
		value = $bindable(''),
		ref = $bindable(null),
		class: className,
		...restProps
	}: CommandPrimitive.RootProps = $props();
</script>

<CommandPrimitive.Root
	class={cn(
		'text-popover-foreground flex h-full w-full flex-col rounded-md border-none',
		className,
	)}
	bind:ref
	bind:value
	{...restProps}
></CommandPrimitive.Root>
