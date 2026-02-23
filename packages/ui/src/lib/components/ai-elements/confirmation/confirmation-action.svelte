<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';

	let {
		children,
		...restProps
	}: ButtonProps & {
		children?: Snippet;
	} = $props();
</script>

<Button
	data-slot="confirmation-action"
	class="h-8 px-3 text-sm"
	type="button"
	{...restProps}
>
	{@render children?.()}
</Button>
