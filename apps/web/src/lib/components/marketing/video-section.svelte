<script lang="ts" module>
	export interface VideoSectionProps {
		title: string;
		subtitle: string;
		videoSrc: string;
	}
</script>

<script lang="ts">
	import { AutoplayVideo } from '@eurora/ui/custom-components/autoplay-video/index';

	let { title, subtitle, videoSrc } = $props();
</script>

<section
	class="relative w-full mx-auto min-h-[80vh] h-fit flex flex-col items-center justify-center overflow-hidden mt-12 gap-6"
>
	<div
		class="flex flex-col h-1/5 w-full text-center text-foreground sm:px-6 lg:px-8 max-w-4xl mx-auto"
	>
		<h1 class="text-4xl w-full m-auto sm:text-5xl lg:text-6xl font-bold mb-4 leading-tight">
			{title}
		</h1>
		<p
			class="text-lg w-full m-auto sm:text-xl lg:text-2xl font-light opacity-90 max-w-3xl mx-auto"
		>
			{subtitle}
		</p>
	</div>
	<div class="flex flex-row h-4/5 w-full p-4">
		<AutoplayVideo src={videoSrc} class="rounded-xl" />
	</div>
</section>
