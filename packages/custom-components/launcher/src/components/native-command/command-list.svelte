<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let { ref = $bindable(null), class: className = '', children, ...restProps } = $props();
</script>

<div
	class={cn('max-h-[300px] overflow-y-auto overflow-x-hidden', className)}
	bind:this={ref}
	role="listbox"
	tabindex="-1"
	data-command-list
	{...restProps}
>
	{@render children?.()}
</div>
