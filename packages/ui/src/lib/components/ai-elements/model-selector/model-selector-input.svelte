<script lang="ts">
	import * as Command from '$lib/components/command/index.js';
	import type { Command as CommandPrimitive } from 'bits-ui';
	import type { WithoutChildrenOrChild } from '$lib/utils.js';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		...restProps
	}: WithoutChildrenOrChild<CommandPrimitive.InputProps> = $props();
</script>

<Command.Input
	bind:ref
	bind:value
	data-slot="model-selector-input"
	class={cn('h-auto py-3.5', className)}
	{...restProps}
/>
