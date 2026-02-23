<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-time-range
	data-slot="audio-player-time-range"
	class={cn('bg-transparent', className)}
	{...restProps}
></media-time-range>
