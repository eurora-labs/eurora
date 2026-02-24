<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu/index.js';

	interface Props {
		class?: string;
	}

	let { ...restProps }: Props = $props();
</script>

<DropdownMenu.Separator data-slot="open-in-separator" {...restProps} />
