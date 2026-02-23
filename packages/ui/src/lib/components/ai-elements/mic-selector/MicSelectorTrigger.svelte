<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { Button } from '$lib/components/button/index.js';
	import * as Popover from '$lib/components/popover/index.js';
	import ChevronsUpDownIcon from '@lucide/svelte/icons/chevrons-up-down';
	import { getMicSelectorContext } from './mic-selector-context.svelte.js';
	import { onMount, onDestroy } from 'svelte';

	interface Props extends HTMLButtonAttributes {
		children?: Snippet;
	}

	let { children, ...restProps }: Props = $props();

	let context = getMicSelectorContext();
	let buttonRef: HTMLButtonElement | undefined = $state();

	let resizeObserver: ResizeObserver | undefined;

	onMount(() => {
		if (buttonRef) {
			resizeObserver = new ResizeObserver((entries) => {
				for (const entry of entries) {
					const newWidth = (entry.target as HTMLElement).offsetWidth;
					if (newWidth) {
						context.width = newWidth;
					}
				}
			});
			resizeObserver.observe(buttonRef);
		}
	});

	onDestroy(() => {
		resizeObserver?.disconnect();
	});
</script>

<Popover.Trigger>
	{#snippet child({ props })}
		<Button data-slot="mic-selector-trigger" variant="outline" {...props} {...restProps} bind:ref={buttonRef}>
			{@render children?.()}
			<ChevronsUpDownIcon class="shrink-0 text-muted-foreground" size={16} />
		</Button>
	{/snippet}
</Popover.Trigger>
