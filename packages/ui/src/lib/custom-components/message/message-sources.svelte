<script lang="ts">
	import { cn, type WithElementRef } from '$lib/utils.js';
	import type { HTMLAttributes } from 'svelte/elements';
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	let { ref = $bindable(null), class: className, children, ...restProps }: ButtonProps = $props();
</script>

<Button
	{ref}
	data-slot="message-sources"
	class={cn('flex items-start', className)}
	variant="ghost"
	size="default"
	{...restProps}
>
	{@render children?.()}
	Sources
</Button>
