<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props {
		class?: string;
		value?: string;
		children?: Snippet;
	}

	let { children, value, class: className }: Props = $props();

	let context = getMicSelectorContext();

	function handleSelect() {
		if (value) {
			context.value = value;
		}
		context.open = false;
	}
</script>

<Command.Item data-slot="mic-selector-item" {value} onSelect={handleSelect} class={className}>
	{@render children?.()}
</Command.Item>
