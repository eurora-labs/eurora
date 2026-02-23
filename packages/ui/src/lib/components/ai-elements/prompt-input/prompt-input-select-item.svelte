<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { SelectItem } from '$lib/components/select/index.js';
	import { Select as SelectPrimitive } from 'bits-ui';
	import type { WithoutChild } from '$lib/utils.js';

	let {
		class: className,
		...restProps
	}: WithoutChild<SelectPrimitive.ItemProps> = $props();
</script>

<SelectItem
	data-slot="prompt-input-select-item"
	class={cn(className)}
	{...restProps}
/>
