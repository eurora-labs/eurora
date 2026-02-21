<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
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

	<div class="w-full rounded-xl overflow-hidden">
		<video
			class="w-full h-auto object-cover"
			src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
			autoplay
			muted
			loop
			playsinline
			preload="auto"
		></video>
	</div>
</div>
