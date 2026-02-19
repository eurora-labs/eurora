<script lang="ts">
	import * as Dialog from '$lib/components/dialog/index.js';
	import * as Command from '$lib/components/command/index.js';
	import type { Dialog as DialogPrimitive } from 'bits-ui';
	import type { WithoutChildrenOrChild } from '$lib/utils.js';
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		portalProps,
		...restProps
	}: WithoutChildrenOrChild<DialogPrimitive.ContentProps> & {
		children: Snippet;
		portalProps?: DialogPrimitive.PortalProps;
	} = $props();
</script>

<Dialog.Content bind:ref class={cn('p-0', className)} {portalProps} {...restProps}>
	<Command.Root class="**:data-[slot=command-input-wrapper]:h-auto">
		{@render children()}
	</Command.Root>
</Dialog.Content>
