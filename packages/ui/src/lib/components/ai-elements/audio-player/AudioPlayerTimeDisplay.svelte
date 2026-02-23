<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-time-display
	data-slot="audio-player-time-display"
	class={cn('bg-transparent tabular-nums', className)}
	{...restProps}
></media-time-display>
