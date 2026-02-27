<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import VideoSection from '$lib/components/marketing/video-section.svelte';
	import { AutoplayVideo } from '@eurora/ui/custom-components/autoplay-video/index';
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

	<AutoplayVideo
		src="https://d26xptavrz5c8t.cloudfront.net/video/youtube_demo.mp4"
		class="rounded-xl"
	/>

	<VideoSection
		title="Seamless Integration"
		subtitle="Eurora works in every browser across MacOS, Windows and Linux."
		videoSrc="https://d26xptavrz5c8t.cloudfront.net/video/juice_demo.mp4"
	></VideoSection>
</div>
