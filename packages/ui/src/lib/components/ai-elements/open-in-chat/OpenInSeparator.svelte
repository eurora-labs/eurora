<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu';

	interface Props {
		class?: string;
	}

	let { ...restProps }: Props = $props();
</script>

<DropdownMenu.Separator {...restProps} />
