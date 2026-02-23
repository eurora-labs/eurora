<script lang="ts">
	import type { Snippet } from 'svelte';

	interface Props {
		placeholder?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { placeholder, children, ...rest }: Props = $props();
</script>

<span data-slot="code-block-language-selector-value" {...rest}>
	{#if children}
		{@render children()}
	{:else if placeholder}
		{placeholder}
	{/if}
</span>
