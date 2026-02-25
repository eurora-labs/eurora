<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { InputGroupAddon } from '$lib/components/input-group/index.js';

	let {
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		children?: Snippet;
	} = $props();
</script>

<InputGroupAddon data-slot="snippet-addon" {...restProps}>
	{@render children?.()}
</InputGroupAddon>
