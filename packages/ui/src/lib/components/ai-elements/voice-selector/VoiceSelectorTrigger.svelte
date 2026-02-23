<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import * as Dialog from '$lib/components/dialog/index.js';

	interface Props extends HTMLButtonAttributes {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<Dialog.Trigger data-slot="voice-selector-trigger" {...restProps}>
	{@render children?.()}
</Dialog.Trigger>
