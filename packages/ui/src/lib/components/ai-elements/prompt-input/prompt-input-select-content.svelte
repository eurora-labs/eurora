<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { SelectContent } from '$lib/components/select/index.js';
	import { Select as SelectPrimitive } from 'bits-ui';

	let {
		class: className,
		...restProps
	}: SelectPrimitive.ContentProps = $props();
</script>

<SelectContent
	data-slot="prompt-input-select-content"
	class={cn(className)}
	{...restProps}
/>
