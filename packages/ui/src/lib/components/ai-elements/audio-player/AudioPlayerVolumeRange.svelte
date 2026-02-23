<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-volume-range
	data-slot="audio-player-volume-range"
	class={cn('bg-transparent', className)}
	{...restProps}
></media-volume-range>
