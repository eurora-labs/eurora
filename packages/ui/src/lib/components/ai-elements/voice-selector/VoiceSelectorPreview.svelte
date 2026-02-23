<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import PlayIcon from '@lucide/svelte/icons/play';
	import PauseIcon from '@lucide/svelte/icons/pause';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';

	interface Props {
		class?: string;
		playing?: boolean;
		loading?: boolean;
		onPlay?: () => void;
		onclick?: (e: MouseEvent) => void;
	}

	let { class: className, playing, loading, onPlay, onclick }: Props = $props();

	function handleClick(event: MouseEvent) {
		event.stopPropagation();
		onclick?.(event);
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
>
	{#if loading}
		<LoaderCircleIcon class="size-3 animate-spin" />
	{:else if playing}
		<PauseIcon class="size-3" />
	{:else}
		<PlayIcon class="size-3" />
	{/if}
</Button>
