<script lang="ts">
	import * as DropdownMenu from '$lib/components/dropdown-menu/index.js';
	import { setOpenInContext } from './open-in-context.svelte.js';
	import { watch } from 'runed';

	interface Props {
		query: string;
		children?: import('svelte').Snippet;
	}

	let { query, children }: Props = $props();

	let contextInstance = setOpenInContext(query);

	watch(
		() => query,
		() => {
			contextInstance.query = query;
		},
	);
</script>

<DropdownMenu.Root>
	{@render children?.()}
</DropdownMenu.Root>
