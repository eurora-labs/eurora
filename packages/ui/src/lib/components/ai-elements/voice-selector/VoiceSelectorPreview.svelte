<script lang="ts">
	import type { HTMLButtonAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import PlayIcon from '@lucide/svelte/icons/play';
	import PauseIcon from '@lucide/svelte/icons/pause';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';

	interface Props extends HTMLButtonAttributes {
		playing?: boolean;
		loading?: boolean;
		onPlay?: () => void;
	}

	let { class: className, playing, loading, onPlay, onclick, ...restProps }: Props = $props();

	function handleClick(event: MouseEvent) {
		event.stopPropagation();
		if (onclick) {
			(onclick as (e: MouseEvent) => void)(event);
		}
		onPlay?.();
	}
</script>

<Button
	data-slot="voice-selector-preview"
	aria-label={playing ? 'Pause preview' : 'Play preview'}
	class={cn('size-6', className)}
	disabled={loading}
	onclick={handleClick}
	size="icon-sm"
	type="button"
	variant="outline"
	{...restProps}
>
	{#if loading}
		<LoaderCircleIcon class="size-3 animate-spin" />
	{:else if playing}
		<PauseIcon class="size-3" />
	{:else}
		<PlayIcon class="size-3" />
	{/if}
</Button>
