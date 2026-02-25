<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { buttonVariants } from '$lib/components/button/index.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-play-button
	data-slot="audio-player-play-button"
	class={cn(buttonVariants({ variant: 'outline', size: 'icon-sm' }), 'bg-transparent', className)}
	{...restProps}
></media-play-button>
