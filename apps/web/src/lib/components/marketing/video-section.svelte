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

<section class="w-full px-4 pt-16">
	<div class="mx-auto mb-6 max-w-4xl text-center">
		<h2 class="mb-4 text-3xl font-bold sm:text-2xl lg:text-5xl">{title}</h2>
		<p class="mx-auto max-w-2xl text-xl text-muted-foreground">{subtitle}</p>
	</div>
	<AutoplayVideo src={videoSrc} class="rounded-xl" />
</section>
