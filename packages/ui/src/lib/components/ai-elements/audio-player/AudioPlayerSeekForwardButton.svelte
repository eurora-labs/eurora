<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {
		seekoffset?: number;
	}

	let { class: className, seekoffset = 10, ...restProps }: Props = $props();
</script>

<media-seek-forward-button
	data-slot="audio-player-seek-forward-button"
	{seekoffset}
	class={cn(
		'inline-flex h-8 w-8 cursor-pointer items-center justify-center rounded-md border bg-transparent text-sm font-medium shadow-xs transition-colors hover:bg-accent hover:text-accent-foreground',
		className,
	)}
	{...restProps}
></media-seek-forward-button>
