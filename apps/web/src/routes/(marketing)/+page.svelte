<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import { Button } from '@eurora/ui/components/button/index';
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
	<title>Eurora — AI Assistant for Your Browser</title>
	<meta
		name="description"
		content="Free, open-source AI assistant that reads what you read. Ask about any YouTube video, article, or tweet without copy-pasting. Private and built in Europe."
	/>
</svelte:head>

<div class="mx-auto w-full max-w-7xl px-4">
	<div class="flex flex-col items-start py-12 max-w-3xl mx-auto">
		<h1 class="text-4xl font-bold text-shadow-xl sm:text-5xl lg:text-6xl">
			Your AI Assistant fully integrated into your browser
		</h1>
		<p class="text-lg text-muted-foreground sm:text-xl max-w-2xl mt-3">
			<span class="text-foreground font-semibold">Less typing, more answers.</span>
			A private, open-source AI assistant that reads what you read. Ask questions about any YouTube
			video, article, or tweet — Eurora captures the transcript, content, and metadata so you don't
			have to copy-paste a thing.
		</p>
		<div class="flex flex-col items-center gap-4 w-full md:flex-row md:items-start mt-16">
			<DownloadButton class="h-24 w-full max-w-md" />
		</div>
	</div>

	{#key activeTab}
		<AutoplayVideo src={active.src} loop={active.loop} class="rounded-xl mt-8" />
	{/key}

	<div class="grid grid-cols-3 gap-3 mt-4">
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

	<div class="flex flex-col gap-24 mt-24">
		<FeatureSection.Root>
			<FeatureSection.Header>
				<FeatureSection.Title>AI On Your Own Terms</FeatureSection.Title>
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
				<FeatureSection.Title
					>Eurora — one application — every platform</FeatureSection.Title
				>
				<FeatureSection.Subtitle>
					Native macOS, Windows, and Linux. Browser extensions include: Chrome, Firefox,
					Edge, Safari and all other browsers. Your preferences, your history, and your AI
					— always with you, always in sync, always accessible, always private.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
		</FeatureSection.Root>
	</div>

	<section
		class="relative mt-32 mb-16 flex flex-col items-center text-center overflow-hidden rounded-3xl border border-primary/10 bg-linear-to-b from-primary/5 via-primary/10 to-primary/5 px-6 py-24 sm:px-12 sm:py-32"
	>
		<div
			class="pointer-events-none absolute inset-0 rounded-3xl bg-[radial-gradient(ellipse_at_center,var(--tw-gradient-stops))] from-primary/15 via-transparent to-transparent"
		></div>

		<h2 class="relative text-4xl font-bold tracking-tight sm:text-5xl lg:text-6xl">
			Ready to get started?
		</h2>
		<p
			class="relative mt-6 max-w-2xl text-lg text-muted-foreground sm:text-xl lg:text-2xl leading-relaxed"
		>
			Download Eurora and experience AI on your own terms. Free, private, and open source.
		</p>
		<Button
			size="lg"
			class="relative mt-10 rounded-full px-12 py-7 text-xl font-semibold shadow-xl shadow-primary/25 hover:shadow-2xl hover:shadow-primary/30 transition-shadow"
			href="/download"
		>
			Download
		</Button>
	</section>
</div>
