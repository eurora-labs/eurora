<script lang="ts">
	import * as Command from '$lib/components/command/index.js';
	import type { Command as CommandPrimitive } from 'bits-ui';
	import type { WithoutChildrenOrChild } from '$lib/utils.js';
	import type { Snippet } from 'svelte';

	let {
		ref = $bindable(null),
		heading,
		children,
		...restProps
	}: WithoutChildrenOrChild<CommandPrimitive.GroupProps> & {
		children: Snippet;
		heading?: string;
	} = $props();
</script>

<Command.Group bind:ref data-slot="model-selector-group" {heading} {...restProps}>
	{@render children()}
</Command.Group>
