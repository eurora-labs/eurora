<script lang="ts">
	import type { Snippet } from 'svelte';
	import { SelectItem } from '$lib/components/select/index.js';

	interface Props {
		value: string;
		label?: string;
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { children, ...rest }: Props = $props();
</script>

<SelectItem data-slot="code-block-language-selector-item" {...rest}>
	{@render children?.()}
</SelectItem>
