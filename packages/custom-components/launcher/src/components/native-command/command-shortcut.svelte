<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let { ref = $bindable(null), class: className = '', children, ...restProps } = $props();
</script>

<span
	class={cn('text-muted-foreground ml-auto text-xs tracking-widest', className)}
	{...restProps}
	bind:this={ref}
	data-command-shortcut
>
	{@render children?.()}
</span>
