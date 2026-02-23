<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		value?: string;
		onSelect?: () => void;
		children?: Snippet;
	}

	let { class: className, children, value, onSelect }: Props = $props();
</script>

<Command.Item data-slot="voice-selector-item" class={cn('px-4 py-2', className)} {value} {onSelect}>
	{@render children?.()}
</Command.Item>
