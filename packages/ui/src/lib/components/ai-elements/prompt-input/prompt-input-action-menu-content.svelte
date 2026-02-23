<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { DropdownMenuContent } from '$lib/components/dropdown-menu/index.js';
	import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';

	let {
		class: className,
		align = 'start',
		...restProps
	}: DropdownMenuPrimitive.ContentProps = $props();
</script>

<DropdownMenuContent
	data-slot="prompt-input-action-menu-content"
	{align}
	class={cn(className)}
	{...restProps}
/>
