<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu/index.js';

	interface Props {
		class?: string;
		children?: import('svelte').Snippet;
	}

	let { children, ...restProps }: Props = $props();
</script>

<DropdownMenu.Label data-slot="open-in-label" {...restProps}>
	{@render children?.()}
</DropdownMenu.Label>
