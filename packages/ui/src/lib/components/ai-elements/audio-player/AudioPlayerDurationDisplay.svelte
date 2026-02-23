<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-duration-display
	data-slot="audio-player-duration-display"
	class={cn('bg-transparent tabular-nums', className)}
	{...restProps}
></media-duration-display>
