<script lang="ts">
	import * as Command from '$lib/components/command/index.js';
	import type { Command as CommandPrimitive } from 'bits-ui';
	import type { WithoutChildrenOrChild } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	let {
		ref = $bindable(null),
		children,
		...restProps
	}: WithoutChildrenOrChild<CommandPrimitive.ListProps> & {
		children: Snippet;
	} = $props();
</script>

<Command.List bind:ref data-slot="model-selector-list" {...restProps}>
	{@render children()}
</Command.List>
