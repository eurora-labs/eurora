<script lang="ts">
	import { Button, type ButtonProps } from '$lib/components/button/index.js';
	import { cn } from '$lib/utils.js';
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
