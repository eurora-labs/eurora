<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { DropdownMenuItem } from '$lib/components/dropdown-menu/index.js';
	import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';

	let { class: className, ...restProps }: DropdownMenuPrimitive.ItemProps = $props();
</script>

<DropdownMenuItem data-slot="prompt-input-action-menu-item" class={cn(className)} {...restProps} />
