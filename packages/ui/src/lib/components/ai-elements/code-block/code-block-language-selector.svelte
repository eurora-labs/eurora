<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Select } from '$lib/components/select/index.js';

	interface Props {
		value?: string;
		onValueChange?: (value: string) => void;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { children, ...rest }: Props = $props();
</script>

<Select data-slot="code-block-language-selector" {...rest}>
	{@render children?.()}
</Select>
