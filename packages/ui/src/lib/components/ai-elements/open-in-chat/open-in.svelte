<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu/index.js';
	import { setOpenInContext } from './open-in-context.svelte.js';

	interface Props {
		query: string;
		children?: import('svelte').Snippet;
	}

	let { query, children }: Props = $props();

	setOpenInContext({ query: () => query });
</script>

<DropdownMenu.Root>
	{@render children?.()}
</DropdownMenu.Root>
