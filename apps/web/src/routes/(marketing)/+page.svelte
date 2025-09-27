<script lang="ts">
	import VideoSection from './video-section.svelte';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	// import System from 'svelte-system-info';
	import * as Card from '@eurora/ui/components/card/index';
	import * as VideoCard from '@eurora/ui/custom-components/video-card/index';
	// import gradient.svg from static folder
	import { type Icon as IconType } from '@lucide/svelte';

	import EyeIcon from '@lucide/svelte/icons/eye';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import ZapIcon from '@lucide/svelte/icons/zap';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import MessageSquareIcon from '@lucide/svelte/icons/message-square';
	import KeyRoundIcon from '@lucide/svelte/icons/key-round';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import GaugeIcon from '@lucide/svelte/icons/gauge';
	import GithubIcon from '@lucide/svelte/icons/github';
	import LayersIcon from '@lucide/svelte/icons/layers';
	import CodeIcon from '@lucide/svelte/icons/code';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import { SiLinux, SiApple } from '@icons-pack/svelte-simple-icons';
	import StaticLauncher from './static-launcher.svelte';

	let inputValue = $state('');
	let purpleText = $state('');
	let formSubmitted = $state(false);

	// Typing animation configuration
	const instantTyping = false;
	const firstPart = 'Explain ';
	const secondPart = 'this';
	const typingSpeed = instantTyping ? 0 : 150; // milliseconds per character
	const initialDelay = instantTyping ? 0 : 50; // milliseconds before typing starts

	let emailField = $state('');

	interface DownloadItem {
		name: string;
		url: string;
		icon?: any;
	}

	let downloads: Record<string, DownloadItem> = {
		linux: {
			name: 'Linux',
			url: '/download/linux',
			icon: SiLinux,
		},
		macos: {
			name: 'macOS',
			url: '/download/macos',
			icon: SiApple,
		},
		windows: {
			name: 'Windows',
			url: '/download/windows',
		},
	};

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

	function submitEmail() {
		fetch(
			'https://api.hsforms.com/submissions/v3/integration/submit/242150186/7b08711a-8657-42a0-932c-0d2c4dbbc0f9',
			{
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify({
					fields: [
						{
							name: 'email',
							value: emailField, // replace with your form data
						},
					],
					context: {
						pageUri: window.location.href,
						pageName: document.title,
					},
				}),
			},
		)
			.then(() => {
				formSubmitted = true;
			})
			.catch((error) => {
				console.error('Error submitting form:', error);
			});
	}
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
				class="backdrop-blur-2xl bg-white/20 rounded-2xl mx-auto w-[50%] min-w-[850px]"
			/>

			<div class="flex flex-1 flex-row w-full justify-center align-start px-4 gap-4 mt-16">
				{#each cards as card}
					{@const Icon = card.icon}
					<Card.Root
						class="card-content flex flex-col bg-white/20 backdrop-blur-2xl border-none w-[20%] min-w-[280px] h-[250px] py-8 justify-start"
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
	<!-- <IntroModule /> -->
	<div class="py-[8rem]">
		<VideoCard.Card class="w-[90%] mx-auto video-card border-white border-1 shadow-none">
			<VideoCard.Content
				alignment="left"
				mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
			>
				<VideoCard.Header>
					<VideoCard.Title>One Click To AI</VideoCard.Title>
					<VideoCard.Description>
						Eurora uses a single interface to help with anything and everything you
						need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

	<div class="py-[8rem]">
		<VideoCard.Card class="w-[90%] mx-auto video-card border-white border-1 shadow-none">
			<VideoCard.Content
				mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
				alignment="right"
			>
				<VideoCard.Header>
					<VideoCard.Title>Up to 98% faster responses</VideoCard.Title>
					<VideoCard.Description>
						Contextual understanding and faster responses.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

	<div class="py-[8rem]">
		<VideoCard.Card class="w-[90%] mx-auto video-card border-white border-1 shadow-none">
			<VideoCard.Content
				mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
			>
				<VideoCard.Header>
					<VideoCard.Title
						>Coming soon: Unified Search Across All Your Files
					</VideoCard.Title>
					<VideoCard.Description>
						Make most of what you own without sharing it with a corporation
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

	<!-- Hero Section -->
	<div class="mb-16 text-center">
		<h1 class="mb-6 text-5xl font-bold">By The People, For The People</h1>
		<Card.Root class="p-6 max-w-[70%] mx-auto">
			<Card.Content>
				<ul class="mb-4 space-y-3">
					<li class="flex items-start">
						<div
							class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
						>
							<span class="text-xs text-purple-600">✓</span>
						</div>
						<span>Get instant explanations on complex topics</span>
					</li>
					<li class="flex items-start">
						<div
							class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
						>
							<span class="text-xs text-purple-600">✓</span>
						</div>
						<span>Get real-time translation of live lectures</span>
					</li>
					<li class="flex items-start">
						<div
							class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
						>
							<span class="text-xs text-purple-600">✓</span>
						</div>
						<span>Visualize homework problems and assignments</span>
					</li>
					<li class="flex items-start">
						<div
							class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
						>
							<span class="text-xs text-purple-600">✓</span>
						</div>
						<span
							>Ask how new knowledge relates to previous concepts you've learned
						</span>
					</li>
				</ul>
			</Card.Content>
		</Card.Root>
		<p class="mx-auto mb-8 max-w-3xl text-xl text-gray-600">
			Eurora AI is fully Open Source and and based in the Netherlands. Enjoy the utmost
			protection offered by the European Union anywhere in the world.
		</p>
		<!-- <div class="flex justify-center gap-4">
            <Sheet.Root>
                <Sheet.Trigger class={buttonVariants({ variant: "default" })}
                  >Join Waitlist</Sheet.Trigger
                >
                <Sheet.Content side="right">

                  <ScrollArea class="h-screen">
                    <WaitlistForm portalId="242150186" formId="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b" region="na2" />
                  </ScrollArea>

                  <Sheet.Footer>
                    <Skeleton class="w-full h-screen" />
                    <Skeleton class="w-full h-screen" />
                    <Skeleton class="w-full h-screen" />

                  </Sheet.Footer>
                </Sheet.Content>
              </Sheet.Root>

		</div> -->
	</div>

	<!-- Feature Highlights -->
	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-3">
		<Card.Root class="p-3 md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<BrainIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Intelligent Understanding</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora understands context and remembers previous conversations, providing more
					accurate and relevant responses than traditional AI assistants.
				</p>
				<!-- <Button variant="link" href="/features" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
			</Card.Content>
		</Card.Root>
		<Card.Root class="p-3 md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<KeyRoundIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Fully Open Source</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora gives you full access to the code that runs on your device and handles
					your data. You can even run both the app and server on your own hardware as well
					as connect LLM's of your choosing.
				</p>
				<!-- <Button variant="link" href="/open-source" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-3 md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ShieldIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Privacy-First Design</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Your data never leaves your device without your permission. Eurora is designed
					with privacy at its core, giving you complete control over your information.
				</p>
				<!-- <Button variant="link" href="/privacy" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
			</Card.Content>
		</Card.Root>

		<!-- <Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Globe class="w-6 h-6 text-purple-600" />
					<Card.Title>Works Everywhere</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Use Eurora across all your devices with perfect synchronization. Available on Windows,
					macOS, Linux, iOS, Android, and as browser extensions.
				</p>
				<Button variant="link" href="/download" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button>
			</Card.Content>
		</Card.Root> -->
	</div>

	<!-- Video Showcase -->
	<!-- <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="left"
			>
				<VideoCard.Header>
					<VideoCard.Title>One Click To AI</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

    <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="right"
			>
				<VideoCard.Header>
					<VideoCard.Title>Up to 98% faster responses</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

    <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="left"
			>
				<VideoCard.Header>
					<VideoCard.Title>Use your own LLM's</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div> -->

	<!-- Use Cases -->
	<div class="mb-16">
		<h2 class="mb-8 text-center text-3xl font-bold">How People Use Eurora</h2>
		<div class="grid grid-cols-1 gap-8 md:grid-cols-2">
			<Card.Root class="p-6">
				<Card.Header>
					<Card.Title>For Learning</Card.Title>
					<Card.Description>Enhance your education and skill development</Card.Description
					>
				</Card.Header>
				<Card.Content>
					<ul class="mb-4 space-y-3">
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Get instant explanations on complex topics</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Get real-time translation of live lectures</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Visualize homework problems and assignments</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span
								>Ask how new knowledge relates to previous concepts you've learned
							</span>
						</li>
					</ul>
				</Card.Content>
			</Card.Root>
			<Card.Root class="p-6">
				<Card.Header>
					<Card.Title>For Work</Card.Title>
					<Card.Description
						>Boost your productivity and streamline workflows</Card.Description
					>
				</Card.Header>
				<Card.Content>
					<ul class="mb-4 space-y-3">
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Ask question about any document you're reading</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Summarize long documents and research papers</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Get short description of the work you did yesterday</span>
						</li>
						<li class="flex items-start">
							<div
								class="mr-2 mt-0.5 flex h-5 w-5 items-center justify-center rounded-full bg-purple-100"
							>
								<span class="text-xs text-purple-600">✓</span>
							</div>
							<span>Integrate with your existing productivity tools</span>
						</li>
					</ul>
				</Card.Content>
			</Card.Root>
		</div>
	</div>

	<h2 class="mb-8 text-3xl font-bold">Features</h2>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class=" md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<BrainIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Advanced AI Understanding</Card.Title>
				</div>
				<Card.Description
					>Supercharged context for lightning-fast responses</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora's intelligent context utilization delivers responses up to 98% faster
					than traditional AI assistants, while maintaining exceptional accuracy and
					relevance to your specific needs.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Context Advantages:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Accelerated response times with smart context prioritization</li>
						<li>
							Efficient processing of previous interactions for near-instant answers
						</li>
						<li>Contextual memory that reduces redundant information processing</li>
						<li>Adaptive learning system that gets faster the more you use it</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>
		<Card.Root class=" md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<GithubIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Fully Open Source Hosting</Card.Title>
				</div>
				<Card.Description>Host Eurora on your own infrastructure</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora is completely open source and can be self-hosted by individuals or
					companies. Take full control of your AI assistant by running it on your own
					hardware, ensuring complete data sovereignty and customization options.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Hosting Benefits:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Complete control over your data and infrastructure</li>
						<li>Customizable deployment options for individuals and enterprises</li>
						<li>No vendor lock-in or subscription fees</li>
						<li>Community-supported deployment guides and documentation</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class=" md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<CodeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Built with Rust</Card.Title>
				</div>
				<Card.Description>Maximum security and safety by design</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora is written in Rust, a language designed for performance, reliability, and
					security. This implementation choice ensures memory safety, eliminates common
					vulnerabilities, and provides robust protection for your sensitive data.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Rust Advantages:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Memory safety without garbage collection</li>
						<li>Thread safety to prevent data races</li>
						<li>Zero-cost abstractions for optimal performance</li>
						<li>Comprehensive compile-time checks to catch errors early</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-3 md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ShieldIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Privacy-First Design</Card.Title>
				</div>
				<Card.Description>Your data stays private and secure</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Unlike other AI assistants, Eurora is designed with privacy at its core. Your
					data never leaves your device without your explicit permission, and you have
					complete control over what information is shared.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Privacy Features:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Local processing for sensitive information</li>
						<li>Granular permission controls for data sharing</li>
						<li>Option to delete conversation history at any time</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ZapIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Lightning-Fast Performance</Card.Title>
				</div>
				<Card.Description>Get answers instantly, even offline</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora is optimized for speed, with local processing capabilities that allow it
					to function even without an internet connection. When online, it leverages cloud
					resources for more complex tasks while maintaining responsiveness.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Performance Highlights:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Instant responses for common queries</li>
						<li>Offline mode for essential functionality</li>
						<li>Optimized resource usage to preserve battery life</li>
						<li>Adaptive processing based on device capabilities</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<GlobeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Cross-Platform Integration</Card.Title>
				</div>
				<Card.Description>Seamless experience across all your devices</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Use Eurora across all your devices with perfect synchronization. Whether you're
					on your phone, tablet, computer, or browser, Eurora provides a consistent
					experience with your preferences and history available everywhere.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Available Platforms:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Windows, macOS, and Linux desktop applications</li>
						<li>iOS and Android mobile apps</li>
						<li>Chrome, Firefox, and Edge browser extensions</li>
						<li>Secure cloud synchronization between devices</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<div class="mb-16">
		<Card.Root class=" md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<LayersIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Powerful Integrations</Card.Title>
				</div>
				<Card.Description
					>Seamless access to your files and productivity tools</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<p class="mb-6 text-gray-600">
					Eurora seamlessly integrates with your file storage systems and productivity
					tools, giving you instant access to your content wherever it lives. Whether your
					files are stored in the cloud or on your device, Eurora can search, analyze, and
					help you work with them efficiently.
				</p>

				<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">File Storage</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>Local device storage</li>
							<li>Google Drive</li>
							<li>Dropbox</li>
							<li>OneDrive</li>
							<li>iCloud</li>
						</ul>
					</div>

					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">Productivity Apps</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>Notion</li>
							<li>Obsidian</li>
							<li>Evernote</li>
							<li>OneNote</li>
							<li>Roam Research</li>
						</ul>
					</div>

					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">Workspace Tools</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>Google Workspace</li>
							<li>Microsoft Office</li>
							<li>Slack</li>
							<li>Trello & Asana</li>
							<li>Airtable</li>
						</ul>
					</div>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<MessageSquareIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Natural Conversations</Card.Title>
				</div>
				<Card.Description>Talk to Eurora like you would to a human</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora's conversational abilities go beyond simple command-response
					interactions. Have natural, flowing conversations with follow-up questions,
					clarifications, and even humor.
				</p>
				<div class="mb-4 rounded-md border border-gray-200 bg-gray-50 p-4">
					<div class="mb-3 flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-gray-300"
						>
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 rounded-md bg-blue-50 p-2">
							Can you help me plan a trip to Japan for next spring?
						</div>
					</div>
					<div class="mb-3 flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-purple-300"
						>
							<span class="text-sm font-medium">AI</span>
						</div>
						<div class="flex-1 rounded-md bg-purple-50 p-2">
							I'd be happy to help plan your Japan trip! What are you most interested
							in experiencing there - culture, food, nature, or something else?
						</div>
					</div>
					<div class="flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-gray-300"
						>
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 rounded-md bg-blue-50 p-2">
							I love food and traditional culture. And I'd like to see cherry
							blossoms.
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="md:p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<CodeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Developer-Friendly</Card.Title>
				</div>
				<Card.Description>Extend Eurora with custom plugins and APIs</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Developers can extend Eurora's capabilities with custom plugins and
					integrations. Our comprehensive API and SDK make it easy to build powerful
					extensions that leverage Eurora's AI capabilities.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Developer Resources:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Open API with comprehensive documentation</li>
						<li>SDK for multiple programming languages</li>
						<li>Plugin marketplace for sharing and discovering extensions</li>
						<li>Developer community and support forums</li>
					</ul>
				</div>
				<div class="mt-4">
					<Button variant="outline" class="w-full">
						<CodeIcon class="mr-2 h-4 w-4" />
						Explore Developer Docs
					</Button>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="p-3 md:p-6">
		<Card.Header>
			<div class="flex items-center gap-2">
				<SparklesIcon class="h-6 w-6 text-purple-600" />
				<Card.Title>Coming Soon</Card.Title>
			</div>
			<Card.Description>Exciting new features on our roadmap</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Advanced Image Generation</h3>
					<p class="text-gray-600">
						Create stunning, customized images from text descriptions with our upcoming
						image generation feature.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Voice Assistant Mode</h3>
					<p class="text-gray-600">
						Interact with Eurora using just your voice, with advanced speech recognition
						and natural responses.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Collaborative Workspaces</h3>
					<p class="text-gray-600">
						Work together with teammates using shared AI workspaces for collaborative
						projects and brainstorming.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Testimonials -->
	<div class="mb-16 hidden p-6">
		<h2 class="mb-8 text-center text-3xl font-bold">
			What Our Users Say About Intelligence Without Compromise
		</h2>
		<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
			<Card.Root class="p-6">
				<Card.Content>
					<p class="mb-4 italic text-gray-600">
						"Eurora has given me a newfound confidence in my ability to use AI. It's
						like having a personal tutor that can explain complex topics in a way that
						makes sense."
					</p>
					<div class="flex items-center">
						<div class="mr-3 h-10 w-10 rounded-full bg-gray-200"></div>
						<div>
							<p class="font-medium">Andre Compton</p>
							<p class="text-sm text-gray-500">Data Analyst</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root class="p-6">
				<Card.Content>
					<p class="mb-4 italic text-gray-600">
						"As a developer, I appreciate how Eurora integrates with all my tools. The
						API is well-documented and the customization options are fantastic."
					</p>
					<div class="flex items-center">
						<div class="mr-3 h-10 w-10 rounded-full bg-gray-200"></div>
						<div>
							<p class="font-medium">Michael Chen</p>
							<p class="text-sm text-gray-500">Software Engineer</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root class="p-6">
				<Card.Content>
					<p class="mb-4 italic text-gray-600">
						"I use Eurora daily for my studies. It helps me understand complex topics
						and saves me hours of research time. The contextual understanding is
						impressive."
					</p>
					<div class="flex items-center">
						<div class="mr-3 h-10 w-10 rounded-full bg-gray-200"></div>
						<div>
							<p class="font-medium">Emily Rodriguez</p>
							<p class="text-sm text-gray-500">Graduate Student</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>
		</div>
	</div>

	<!-- Call to Action -->
	<Card.Root class="hidden border-none bg-purple-50 p-8">
		<Card.Content class="text-center">
			<h2 class="mb-4 text-3xl font-bold">
				Ready to Experience Intelligence Without Compromise?
			</h2>
			<p class="mx-auto mb-6 max-w-2xl text-xl text-gray-600">
				Join thousands of users who are already experiencing the future of AI assistance.
			</p>
			<div class="flex justify-center gap-4">
				<Button href="/download" size="lg" class="px-8">Download Now</Button>
				<Button href="/pricing" variant="outline" size="lg" class="px-8"
					>View Pricing</Button
				>
			</div>
		</Card.Content>
	</Card.Root>
</div>

<style lang="postcss">
	/* Optional: Add a blinking cursor animation */
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
	}

	:global(.animate-blink) {
		animation: blink 1s infinite;
	}

	@keyframes grow {
		from {
			transform: scale(0.2);
		}
		to {
			transform: scale(1);
		}
	}

	.animate-grow {
		animation: grow var(--animation-duration) ease-in-out;
	}

	:global(.animate-grow) {
		--animation-duration: 300ms;
	}

	.card-entrance {
		transform: translateY(20px);
		animation: slideIn 0.5s ease-out forwards;
		opacity: 0;
		/* Add transition for smoother repositioning if needed */
		transition: all 0.3s ease-out;
	}

	@keyframes slideIn {
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	:global(.video-card) {
		/* background: #0e2040;
		background: #061225;
		background-image: radial-gradient(#fff 1px, transparent 1px);
		background-image: radial-gradient(#061225 1px, transparent 1px);
		background-size: 60px 60px; */
		background:
			linear-gradient(0deg, #19366b 1px, transparent 1px),
			linear-gradient(90deg, #19366b 1px, transparent 1px), #122547;
		background-size: 60px 60px;
	}

	:global(.download-button-container) {
		& svg {
			width: 24px;
			height: 24px;
		}
	}
</style>
