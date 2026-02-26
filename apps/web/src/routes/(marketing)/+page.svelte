<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import VideoSection from '$lib/components/marketing/video-section.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import PlayIcon from '@lucide/svelte/icons/play';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';

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

<div class="container mx-auto px-4 py-8">
	<div class="flex flex-col items-start gap-4 mb-8">
		<h1 class="text-2xl font-bold text-shadow-xl">
			Never explain yourself again,
			<br />
			Eurora is the easiest way to use AI.
		</h1>
		<DownloadButton class="rounded-full" />
	</div>

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="relative w-full rounded-xl overflow-hidden cursor-pointer" onclick={play}>
		<video
			use:autoplay
			class="w-full h-auto object-cover"
			src="https://d26xptavrz5c8t.cloudfront.net/video/youtube_demo.webm"
			autoplay
			muted
			playsinline
			preload="auto"
		></video>
		{#if paused && !ended}
			<div class="absolute inset-0 flex items-center justify-center bg-black/30">
				<PlayIcon class="size-16 text-background drop-shadow-lg" />
			</div>
		{/if}
		{#if ended}
			<Button variant="outline" size="icon-sm" class="absolute top-3 left-3">
				<RotateCcwIcon />
			</Button>
		{/if}
	</div>

	<VideoSection
		title="test"
		subtitle="test"
		videoSrc="https://d26xptavrz5c8t.cloudfront.net/video/youtube_demo.webm"
	></VideoSection>
</div>
