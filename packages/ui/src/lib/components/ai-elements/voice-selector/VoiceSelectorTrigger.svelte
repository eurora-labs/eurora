<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Dialog from '$lib/components/dialog/index.js';

	interface Props {
		class?: string;
		children?: Snippet;
	}

	let { children, class: className }: Props = $props();
</script>

<Dialog.Trigger data-slot="voice-selector-trigger" class={className}>
	{@render children?.()}
</Dialog.Trigger>
