<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import VideoSection from '$lib/components/marketing/video-section.svelte';
	import { AutoplayVideo } from '@eurora/ui/custom-components/autoplay-video/index';
	import * as FeatureSection from '@eurora/ui/custom-components/feature-section/index';
	import { Button } from '@eurora/ui/components/button/index';
</script>

<div class="container mx-auto px-4 py-8">
	<div class="flex flex-col items-start gap-4 mb-8">
		<h1 class="text-3xl font-bold text-shadow-xl">
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
	<p class="mt-2 text-xs text-muted-foreground/70">
		Video source: <a
			href="https://www.youtube.com/watch?v=zkIcLproDGM"
			target="_blank"
			rel="noopener noreferrer"
			class="underline hover:text-muted-foreground transition-colors"
			>Lecture 1 Part 1: Introduction and Motivation</a
		> — Alan Edelman, Steven G. Johnson | OCW MIT
	</p>

	<VideoSection
		title="Seamless Integration"
		subtitle="Eurora works in every browser across MacOS, Windows and Linux."
		videoSrc="https://d26xptavrz5c8t.cloudfront.net/video/juice_demo.mp4"
	></VideoSection>

	<div class="flex flex-col gap-24 mt-24">
		<FeatureSection.Root>
			<FeatureSection.Header>
				<FeatureSection.Title>AI On Your Own Terms</FeatureSection.Title>
				<FeatureSection.Subtitle>
					Get answers instantly, everywhere. Eurora works on your platform, with your
					browser. All data is stored securely in a Soveign European data center.
					Regardless of your location. Enjoy the highest standard of Data Protection and
					Privacy.
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

		<FeatureSection.Root align="right">
			<FeatureSection.Header>
				<FeatureSection.Title>European Open Source AI Platform</FeatureSection.Title>
				<FeatureSection.Subtitle>
					Use it locally for free forever, or make use of secure European Cloud. We are
					self-funded and would appreciate your subscription for just €10 for the first
					month.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
			<FeatureSection.Content class="overflow-hidden">
				<p
					class="flex items-center p-6 h-200 text-lg leading-relaxed text-muted-foreground sm:p-10 sm:text-xl lg:p-14"
				>
					We are fully Open Source. Every single line of code is available for you to see.
					You do not have to trust us. We do everything we can to provide you with the
					highest standard of security and privacy. And if that is not enough, we will
					support you fully in running Eurora fully locally.
				</p>
			</FeatureSection.Content>
		</FeatureSection.Root>

		<FeatureSection.Root>
			<FeatureSection.Header>
				<FeatureSection.Title></FeatureSection.Title>
				<FeatureSection.Subtitle>
					One app. Every platform. No compromises.
				</FeatureSection.Subtitle>
			</FeatureSection.Header>
			<FeatureSection.Content>
				<p
					class="p-6 text-lg leading-relaxed text-muted-foreground sm:p-10 sm:text-xl lg:p-14"
				>
					Native on MacOS, Windows, and Linux. Browser extensions for Chrome, Firefox,
					Edge, Safari and every other browser. Your preferences, your history, and your
					AI — always with you, always in sync, always private. One download and you're
					set.
				</p>
			</FeatureSection.Content>
		</FeatureSection.Root>
	</div>

	<section
		class="relative mt-32 mb-16 flex flex-col items-center text-center overflow-hidden rounded-3xl border border-primary/10 bg-gradient-to-b from-primary/5 via-primary/10 to-primary/5 px-6 py-24 sm:px-12 sm:py-32"
	>
		<div
			class="pointer-events-none absolute inset-0 rounded-3xl bg-[radial-gradient(ellipse_at_center,_var(--tw-gradient-stops))] from-primary/15 via-transparent to-transparent"
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
			Get Started
		</Button>
	</section>
</div>
