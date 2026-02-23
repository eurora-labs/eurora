<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';

	let {
		class: className,
		children,
		...restProps
	}: ButtonProps & {
		children?: Snippet;
	} = $props();
</script>

<Button
	data-slot="confirmation-action"
	class={cn('h-8 px-3 text-sm', className)}
	type="button"
	{...restProps}
>
	{@render children?.()}
</Button>
