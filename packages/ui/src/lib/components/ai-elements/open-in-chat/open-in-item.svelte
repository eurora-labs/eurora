<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu/index.js';

	interface Props {
		class?: string;
		children?: import('svelte').Snippet;
		onclick?: () => void;
	}

	let { children, ...restProps }: Props = $props();
</script>

<DropdownMenu.Item data-slot="open-in-item" {...restProps}>
	{@render children?.()}
</DropdownMenu.Item>
