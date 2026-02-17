<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import StaticLauncher from '$lib/components/marketing/static-launcher.svelte';
	import VideoSection from '$lib/components/marketing/video-section.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { type Icon as IconType } from '@lucide/svelte';

	import EyeIcon from '@lucide/svelte/icons/eye';
	import GaugeIcon from '@lucide/svelte/icons/gauge';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';

	interface CardItem {
		icon: typeof IconType;
		title: string;
		description: string;
	}

	let cards = $state<CardItem[]>([
		{
			icon: EyeIcon,
			title: 'Context aware',
			description: 'Stop explaining yourself and start asking',
		},
		{
			icon: GaugeIcon,
			title: 'Extremely fast',
			description: 'Up to 17x faster answers compared to using traditional LLM interfaces',
		},
		{
			icon: ShieldCheckIcon,
			title: 'Can be run locally',
			description: 'For free, forever',
		},
	]);
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
