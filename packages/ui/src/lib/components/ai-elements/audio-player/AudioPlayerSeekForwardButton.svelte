<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { buttonVariants } from '$lib/components/button/index.js';

	interface Props extends HTMLAttributes<HTMLElement> {
		seekoffset?: number;
	}

	let { class: className, seekoffset = 10, ...restProps }: Props = $props();
</script>

<media-seek-forward-button
	data-slot="audio-player-seek-forward-button"
	{seekoffset}
	class={cn(buttonVariants({ variant: 'outline', size: 'icon-sm' }), className)}
	{...restProps}
></media-seek-forward-button>
