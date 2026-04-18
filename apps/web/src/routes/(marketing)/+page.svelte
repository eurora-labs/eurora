<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import ReadyToStart from '$lib/components/marketing/ReadyToStart.svelte';
	import * as Card from '@eurora/ui/components/card/index';
	import { AutoplayVideo } from '@eurora/ui/custom-components/autoplay-video/index';
	import * as FeatureSection from '@eurora/ui/custom-components/feature-section/index';

	const tabs = [
		{
			id: 'first',
			label: 'Context recognition',
			description: 'Instant context from your browser that you can ask about',
			src: 'https://d26xptavrz5c8t.cloudfront.net/video/youtube_demo.mp4',
			loop: false,
		},
		{
			id: 'second',
			label: 'Instantly works on every website',
			description: 'Eurora instantly answers your questions.',
			src: 'https://d26xptavrz5c8t.cloudfront.net/video/multiple_websites_demo.mp4',
			loop: true,
		},
		{
			id: 'third',
			label: 'The last AI assistant you will ever need',
			description: 'A single AI assistant that works everywhere',
			src: 'https://d26xptavrz5c8t.cloudfront.net/video/twitter_demo.mp4',
			loop: false,
		},
	] as const;

	let activeTab = $state<(typeof tabs)[number]['id']>('first');
	let active = $derived(tabs.find((t) => t.id === activeTab)!);
</script>

<svelte:head>
	<title>Eurora - AI Assistant for Your Browser</title>
	<meta
		name="description"
		content="Free, open-source AI assistant that reads what you read. Ask about any YouTube video, article, or tweet without copy and paste. Private and built in Europe."
	/>
</svelte:head>

<div class="px-4 pt-16 pb-16 flex flex-col gap-24">
	<div class="flex flex-col items-start max-w-3xl mx-auto">
		<h1 class="text-4xl font-bold text-shadow-xl sm:text-5xl lg:text-6xl">
			Your AI Assistant fully integrated into your browser
		</h1>
		<p class="text-lg text-muted-foreground sm:text-xl max-w-2xl mt-3">
			<span class="text-foreground font-semibold">Less typing, more answers.</span>
			A private, open-source AI assistant that reads what you read. Ask questions about any YouTube
			video, article, or tweet. Eurora captures the transcript, content, and metadata so you don't
			have to copy and paste a thing.
		</p>
		<div class="flex flex-col items-center gap-4 w-full md:flex-row md:items-start mt-16">
			<DownloadButton class="h-24 w-full max-w-md" />
		</div>
	</div>

	<div class="flex flex-col gap-4">
		{#key activeTab}
			<AutoplayVideo src={active.src} loop={active.loop} class="rounded-xl" />
		{/key}

		<div class="grid grid-cols-3 gap-3">
			{#each tabs as tab}
				<button type="button" class="text-left" onclick={() => (activeTab = tab.id)}>
					<Card.Root
						class="h-full cursor-pointer transition-colors py-3 {activeTab === tab.id
							? 'border-primary bg-primary/5'
							: 'hover:border-muted-foreground/25'}"
					>
						<Card.Header class="px-3 py-0 sm:px-4 gap-0.5">
							<Card.Title class="text-xs sm:text-sm">{tab.label}</Card.Title>
							<Card.Description class="text-xs hidden sm:block"
								>{tab.description}</Card.Description
							>
						</Card.Header>
					</Card.Root>
				</button>
			{/each}
		</div>
	</div>

	<div class="flex flex-col gap-24">
		<FeatureSection.Root>
			<FeatureSection.Header>
				<FeatureSection.Title>New way to use AI</FeatureSection.Title>
				<FeatureSection.Subtitle>
					Eurora is built with a purpose to make an AI assistant that feel natural and
					ergonomic. With a single click of a button, you can see every single chat as a
					graph of edited messages. Navigate old conversations quickly and easily.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
			<FeatureSection.Content class="overflow-hidden">
				<img
					src="https://d26xptavrz5c8t.cloudfront.net/image/rust_course_graph_view.png"
					alt="Eurora graph view"
					class="w-full"
				/>
			</FeatureSection.Content>
		</FeatureSection.Root>

		<FeatureSection.Root align="right">
			<FeatureSection.Header>
				<FeatureSection.Title>Easiest way to use AI</FeatureSection.Title>
				<FeatureSection.Subtitle>
					Get answers instantly. Eurora works on your platform, with your browser. All
					data is stored securely in a Sovereign European data center. Eurora provides
					independence and accessibility with the highest standard of data protection and
					privacy.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
			<FeatureSection.Content class="overflow-hidden">
				<img
					src="https://d26xptavrz5c8t.cloudfront.net/image/zeroize_enum_explanation.png"
					alt="Zeroize enum explanation"
					class="w-full"
				/>
			</FeatureSection.Content>
		</FeatureSection.Root>

		<FeatureSection.Root>
			<FeatureSection.Header>
				<FeatureSection.Title>Integrates with every browser</FeatureSection.Title>
				<FeatureSection.Subtitle>
					Eurora does not have vendor lock-in. You can use any browser, on any platform.
					You can even connect any local AI model to it. Eurora is made with a certainty
					of transparency and putting your needs first.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
			<FeatureSection.Content class="overflow-hidden">
				<img
					src="https://d26xptavrz5c8t.cloudfront.net/image/eurora_and_browsers.png"
					alt="Eurora and browsers"
					class="w-full"
				/>
			</FeatureSection.Content>
		</FeatureSection.Root>
	</div>

	<ReadyToStart />
</div>
