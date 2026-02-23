<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Command from '$lib/components/command/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { children, class: className }: Props = $props();
</script>

<Command.Empty data-slot="voice-selector-empty" class={className}>
	{@render children?.()}
</Command.Empty>
