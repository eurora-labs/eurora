<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import * as Popover from '$lib/components/popover/index.js';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { class: className, children }: Props = $props();

	let context = getMicSelectorContext();
</script>

<Popover.Content
	data-slot="mic-selector-content"
	class={cn('p-0', className)}
	style="width: {context.width}px"
>
	<Command.Root>
		{@render children?.()}
	</Command.Root>
</Popover.Content>
