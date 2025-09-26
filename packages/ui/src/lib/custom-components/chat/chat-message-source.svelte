<script lang="ts">
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	let { ref = $bindable(null), class: className, children, ...restProps }: ButtonProps = $props();
</script>

<Button
	{ref}
	data-slot="message-source"
	class={cn('flex items-center px-6', className)}
	variant="ghost"
	size="default"
	{...restProps}
>
	{@render children?.()}
	Sources
</Button>
