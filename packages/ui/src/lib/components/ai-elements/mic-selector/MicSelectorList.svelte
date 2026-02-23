<script lang="ts">
	import type { Snippet } from 'svelte';
	import * as Command from '$lib/components/command/index.js';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';

	interface Props {
		class?: string;
		children?: Snippet<[MediaDeviceInfo[]]>;
	}

	let { children, class: className }: Props = $props();

	let context = getMicSelectorContext();
</script>

<Command.List data-slot="mic-selector-list" class={className}>
	{@render children?.(context.devices)}
</Command.List>
