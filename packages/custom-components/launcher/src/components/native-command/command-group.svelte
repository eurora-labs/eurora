<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className = '',
		heading = '',
		children,
		...restProps
	} = $props();
</script>

<div
	class={cn('text-foreground overflow-hidden p-1', className)}
	bind:this={ref}
	role="group"
	aria-labelledby={heading ? 'group-heading' : undefined}
	data-command-group
	{...restProps}
>
	{#if heading}
		<div class="px-2 py-1.5 text-xs font-medium text-black/50" id="group-heading">
			{heading}
		</div>
	{/if}
	<div class="text-black/80">
		{@render children?.()}
	</div>
</div>
