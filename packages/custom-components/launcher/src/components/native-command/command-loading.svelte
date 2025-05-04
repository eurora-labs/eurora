<script lang="ts">
	import { cn } from '@eurora/ui/utils.js';

	let { ref = $bindable(null), class: className = '', children, ...restProps } = $props();
</script>

<div
	class={cn('py-6 text-center text-sm', className)}
	bind:this={ref}
	role="status"
	aria-live="polite"
	data-command-loading
	{...restProps}
>
	{@render children?.()}
	<!-- 
	<slot>
		<div class="flex justify-center">
			<div class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
		</div>
		<div class="mt-2">Loading...</div>
	</slot> -->
</div>
