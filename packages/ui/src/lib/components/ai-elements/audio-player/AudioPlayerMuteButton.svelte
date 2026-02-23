<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	interface Props extends HTMLAttributes<HTMLElement> {}

	let { class: className, ...restProps }: Props = $props();
</script>

<media-mute-button
	data-slot="audio-player-mute-button"
	class={cn('bg-transparent', className)}
	{...restProps}
></media-mute-button>
