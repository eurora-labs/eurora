<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let { ref = $bindable(null), class: className = '', ...restProps } = $props();
</script>

<div
	bind:this={ref}
	class={cn('bg-border -mx-1 h-px', className)}
	role="separator"
	aria-orientation="horizontal"
	data-command-separator
	{...restProps}
></div>
