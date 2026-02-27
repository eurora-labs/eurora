<script lang="ts">
	import { Button } from '$lib/components/button/index.js';
	import { cn } from '$lib/utils.js';
	import PlayIcon from '@lucide/svelte/icons/play';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';

	export interface AutoplayVideoProps {
		src: string;
		class?: string;
	}

	let { src, class: className }: AutoplayVideoProps = $props();

	let paused = $state(true);
	let ended = $state(false);

	function onPlay() {
		paused = false;
		ended = false;
	}

	function onPause() {
		paused = true;
	}

	function onEnded() {
		ended = true;
	}

	function autoplay(node: HTMLVideoElement) {
		node.muted = true;
		node.play()
			.then(onPlay)
			.catch(() => {});

		node.addEventListener('play', onPlay);
		node.addEventListener('pause', onPause);
		node.addEventListener('ended', onEnded);

		return {
			destroy() {
				node.removeEventListener('play', onPlay);
				node.removeEventListener('pause', onPause);
				node.removeEventListener('ended', onEnded);
			},
		};
	}

	function play(e: MouseEvent) {
		const video = (e.currentTarget as HTMLElement).querySelector('video');
		if (!video) return;
		if (ended) video.currentTime = 0;
		video.play();
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class={cn('relative w-full overflow-hidden cursor-pointer', className)} onclick={play}>
	<video
		use:autoplay
		class="w-full h-auto object-cover"
		{src}
		autoplay
		muted
		playsinline
		preload="auto"
	></video>
	{#if paused && !ended}
		<div class="absolute inset-0 flex items-center justify-center bg-black/30">
			<PlayIcon class="size-16 text-white drop-shadow-lg" />
		</div>
	{/if}
	{#if ended}
		<Button variant="outline" size="icon-sm" class="absolute top-3 left-3 backdrop-blur-2xl">
			<RotateCcwIcon />
		</Button>
	{/if}
</div>
