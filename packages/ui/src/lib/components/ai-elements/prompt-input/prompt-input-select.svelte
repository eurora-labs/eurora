<script lang="ts">
	import { Select } from '$lib/components/select/index.js';
	import { Select as SelectPrimitive } from 'bits-ui';

	let { ...restProps }: SelectPrimitive.RootProps = $props();
</script>

<Select {...restProps} />
