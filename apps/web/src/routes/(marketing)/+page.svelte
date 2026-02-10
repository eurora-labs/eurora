<script lang="ts">
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

<div class="mx-auto w-full h-full px-4 pb-16">
	<div class="flex justify-center px-4 h-[5%] items-center">
		<h1
			class="w-full mx-auto text-3xl sm:text-4xl md:text-5xl font-bold text-shadow-xl text-center"
		>
			Your Open Source AI Assistant
		</h1>
	</div>

	<div class="relative mx-auto max-w-[95%] h-[80vh] overflow-hidden rounded-[36px] p-0">
		<div
			class="h-screen flex flex-col w-full mx-auto mt-8 rounded-[36px]"
			style="background-image: url('/images/promo.webp'); background-size: cover; background-position: center; background-repeat: no-repeat;"
		>
			<div class="flex justify-center align-start px-4 gap-4 my-8 download-button-container">
				<!-- {#snippet downloadButtonSnippet()}
					{@const downloadButton = downloads[System.OSName.toLowerCase()]}

					<Button size="lg" class="w-full md:w-auto p-8 shadow-lg gap-4">
						{#if downloadButton.icon}
							{@const Icon = downloadButton.icon}
							<Icon size={48} />
						{/if}
						Download for {downloadButton.name}
					</Button>
				{/snippet} -->
				<!-- {@render downloadButtonSnippet()} -->
				<Button size="lg" class=" md:w-auto p-8 shadow-lg" variant="secondary"
					>Learn More</Button
				>
			</div>
			<StaticLauncher
				class="backdrop-blur-2xl bg-white/20 rounded-2xl mx-auto w-[50%] min-w-212.5"
			/>

			<div class="flex flex-1 flex-row w-full justify-center align-start px-4 gap-4 mt-16">
				{#each cards as card}
					{@const Icon = card.icon}
					<Card.Root
						class="card-content flex flex-col bg-white/20 backdrop-blur-2xl border-none w-[20%] min-w-70 h-62.5 py-8 justify-start"
					>
						<Card.Header class="flex flex-col  items-start justify-center">
							<Card.Title
								class="title-animation text-white text-xl font-semibold flex flex-row items-center gap-4"
							>
								<Icon size={48} />
								{card.title}
							</Card.Title>
							<Card.Description
								class="text-white/80 text-lg font-thin flex flex-row justify-start pt-4"
							>
								{card.description}
							</Card.Description>
						</Card.Header>
					</Card.Root>
				{/each}
			</div>
		</div>
	</div>

	<VideoSection
		title="One Click To AI"
		subtitle="Eurora uses a single interface to help with anything and everything you need."
		videoSrc="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
	/>
	<VideoSection
		title="Up to 98% Faster"
		subtitle="Contextual understanding and faster responses."
		videoSrc="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
	/>
	<VideoSection
		title="Coming Soon"
		subtitle="Unified search across all your files."
		videoSrc="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
	/>
</div>
