<script lang="ts" module>
	export interface VideoSectionProps {
		title: string;
		subtitle: string;
		videoSrc: string;
	}
</script>

<script lang="ts">
	let { title, subtitle, videoSrc } = $props();
</script>

<section
	class="relative w-[90%] mx-auto min-h-[80vh] h-fit flex flex-col items-center justify-center overflow-hidden mt-12 gap-6"
>
	<div
		class="flex flex-col h-1/5 w-full text-center text-white px-4 sm:px-6 lg:px-8 max-w-4xl mx-auto"
	>
		<h1 class="text-4xl w-full m-auto sm:text-5xl lg:text-6xl font-bold mb-4 leading-tight">
			{title}
		</h1>
		<p
			class="text-lg w-full m-auto sm:text-xl lg:text-2xl font-light opacity-90 max-w-2xl mx-auto"
		>
			{subtitle}
		</p>
	</div>
	<div class="flex flex-row h-4/5 w-full">
		<video
			class="w-full h-full object-cover p-4 rounded-[36px]"
			src={videoSrc}
			autoplay
			muted
			loop
			playsinline
			preload="auto"
		></video>
	</div>
</section>
