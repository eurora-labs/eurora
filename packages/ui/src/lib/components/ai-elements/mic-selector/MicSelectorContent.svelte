<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import * as Popover from '$lib/components/popover/index.js';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet;
	}

	let { class: className, children, ...restProps }: Props = $props();

	let context = getMicSelectorContext();
</script>

<Popover.Content data-slot="mic-selector-content" class={cn('p-0', className)} style="width: {context.width}px" {...restProps}>
	<Command.Root>
		{@render children?.()}
	</Command.Root>
</Popover.Content>
