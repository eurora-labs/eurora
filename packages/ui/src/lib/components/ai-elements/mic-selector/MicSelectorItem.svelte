<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		value?: string;
		children?: Snippet;
	}

	let { children, value, ...restProps }: Props = $props();

	let context = getMicSelectorContext();

	function handleSelect(currentValue: string) {
		context.value = currentValue;
		context.open = false;
	}
</script>

<Command.Item data-slot="mic-selector-item" {value} onSelect={handleSelect} {...restProps}>
	{@render children?.()}
</Command.Item>
