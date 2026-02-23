<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import * as Dialog from '$lib/components/dialog/index.js';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		title?: string;
		children?: Snippet;
	}

	let { class: className, children, title = 'Voice Selector', ...restProps }: Props = $props();
</script>

<Dialog.Content data-slot="voice-selector-content" class={cn('p-0', className)} {...restProps}>
	<Dialog.Title class="sr-only">{title}</Dialog.Title>
	<Command.Root class="**:data-[slot=command-input-wrapper]:h-auto">
		{@render children?.()}
	</Command.Root>
</Dialog.Content>
