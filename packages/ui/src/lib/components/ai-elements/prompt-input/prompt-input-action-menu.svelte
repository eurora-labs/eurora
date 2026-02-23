<script lang="ts">
	import { DropdownMenu } from '$lib/components/dropdown-menu/index.js';
	import { DropdownMenu as DropdownMenuPrimitive } from 'bits-ui';

	let { ...restProps }: DropdownMenuPrimitive.RootProps = $props();
</script>

<DropdownMenu {...restProps} />
