<script lang="ts">
	import type { Snippet } from 'svelte';
	import { SelectContent } from '$lib/components/select/index.js';

	interface Props {
		align?: 'start' | 'center' | 'end';
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { align = 'end', children, ...rest }: Props = $props();
</script>

<SelectContent data-slot="code-block-language-selector-content" {align} {...rest}>
	{@render children?.()}
</SelectContent>
