<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { SelectTrigger } from '$lib/components/select/index.js';
	import { Select as SelectPrimitive } from 'bits-ui';
	import type { WithoutChild } from '$lib/utils.js';

	let {
		class: className,
		...restProps
	}: WithoutChild<SelectPrimitive.TriggerProps> = $props();
</script>

<SelectTrigger
	data-slot="prompt-input-select-trigger"
	class={cn(
		'border-none bg-transparent font-medium text-muted-foreground shadow-none transition-colors',
		'hover:bg-accent hover:text-foreground aria-expanded:bg-accent aria-expanded:text-foreground',
		className,
	)}
	{...restProps}
/>
