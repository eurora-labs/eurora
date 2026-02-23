<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import * as Command from '$lib/components/command/index.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		value?: string;
		children?: Snippet;
	}

	let { class: className, children, value, ...restProps }: Props = $props();
</script>

<Command.Item data-slot="voice-selector-item" class={cn('px-4 py-2', className)} {value} {...restProps}>
	{@render children?.()}
</Command.Item>
