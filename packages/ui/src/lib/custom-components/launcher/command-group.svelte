<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		heading,
		...restProps
	}: CommandPrimitive.GroupProps & {
		heading?: string;
	} = $props();
</script>

<CommandPrimitive.Group
	class={cn('text-foreground overflow-hidden p-1', className)}
	bind:ref
	{...restProps}
>
	{#if heading}
		<CommandPrimitive.GroupHeading class="px-2 py-1.5 text-xs font-medium text-black/50">
			{heading}
		</CommandPrimitive.GroupHeading>
	{/if}
	<CommandPrimitive.GroupItems class="text-black/80" {children} />
</CommandPrimitive.Group>
