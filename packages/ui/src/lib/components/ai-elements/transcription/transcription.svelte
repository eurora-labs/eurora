<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { setTranscription } from './transcription-context.svelte.js';

	let {
		class: className,
		currentTime = 0,
		onSeek,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		currentTime?: number;
		onSeek?: (time: number) => void;
		children?: Snippet;
	} = $props();

	setTranscription({
		currentTime: () => currentTime,
		onSeek: () => onSeek,
	});
</script>

<div
	data-slot="transcription"
	class={cn('flex flex-wrap gap-1 leading-relaxed', className)}
	{...restProps}
>
	{@render children?.()}
</div>
