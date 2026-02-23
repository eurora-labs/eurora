<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';

	interface Props {
		class?: string;
		children?: Snippet;
		[key: string]: unknown;
	}

	let { class: className, children, ...rest }: Props = $props();
</script>

<span
	data-slot="code-block-filename"
	class={cn('font-mono', className)}
	{...rest}
>
	{@render children?.()}
</span>
