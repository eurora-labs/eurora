<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let { ref = $bindable(null), class: className = '', children, ...restProps } = $props();
</script>

<div
	class={cn('py-6 text-center text-sm', className)}
	bind:this={ref}
	role="presentation"
	data-command-empty
	{...restProps}
>
	{@render children?.()}
</div>
