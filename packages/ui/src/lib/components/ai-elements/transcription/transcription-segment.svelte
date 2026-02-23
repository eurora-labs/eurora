<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { useTranscription } from './transcription-context.svelte.js';

	let {
		class: className,
		text,
		startSecond,
		endSecond,
		index,
		onclick,
		...restProps
	}: Omit<HTMLAttributes<HTMLButtonElement>, 'onclick'> & {
		text: string;
		startSecond: number;
		endSecond: number;
		index: number;
		onclick?: (event: MouseEvent) => void;
	} = $props();

	const ctx = useTranscription();

	let isActive = $derived(ctx.currentTime >= startSecond && ctx.currentTime < endSecond);
	let isPast = $derived(ctx.currentTime >= endSecond);
</script>

<button
	data-slot="transcription-segment"
	data-active={isActive}
	data-index={index}
	type="button"
	class={cn(
		'inline text-left',
		isActive && 'text-primary',
		isPast && 'text-muted-foreground',
		!(isActive || isPast) && 'text-muted-foreground/60',
		ctx.onSeek && 'cursor-pointer hover:text-foreground',
		!ctx.onSeek && 'cursor-default',
		className
	)}
	onclick={(e) => {
		if (ctx.onSeek) {
			ctx.onSeek(startSecond);
		}
		onclick?.(e);
	}}
	{...restProps}
>
	{text}
</button>
