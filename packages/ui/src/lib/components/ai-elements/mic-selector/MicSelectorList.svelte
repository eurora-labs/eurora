<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props extends HTMLAttributes<HTMLDivElement> {
		children?: Snippet<[MediaDeviceInfo[]]>;
	}

	let { children, ...restProps }: Props = $props();

	let context = getMicSelectorContext();
</script>

<Command.List data-slot="mic-selector-list" {...restProps}>
	{@render children?.(context.devices)}
</Command.List>
