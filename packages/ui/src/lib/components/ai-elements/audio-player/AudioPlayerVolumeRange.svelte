<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();

	const buttonGroupTextBase =
		"flex items-center gap-2 rounded-md border px-4 text-sm font-medium shadow-xs [&_svg]:pointer-events-none [&_svg:not([class*='size-'])]:size-4";
</script>

<media-volume-range
	data-slot="audio-player-volume-range"
	class={cn(buttonGroupTextBase, 'bg-transparent', className)}
	{...restProps}
></media-volume-range>
