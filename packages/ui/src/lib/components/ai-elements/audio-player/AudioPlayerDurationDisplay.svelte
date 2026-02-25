<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();

	const buttonGroupTextBase =
		"flex items-center gap-2 rounded-md border px-4 text-sm font-medium shadow-xs [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4";
</script>

<media-duration-display
	data-slot="audio-player-duration-display"
	class={cn(buttonGroupTextBase, 'bg-transparent tabular-nums', className)}
	{...restProps}
></media-duration-display>
