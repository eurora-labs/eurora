<script lang="ts">
	// Removed Input import as we're using a custom div
	import { onMount } from 'svelte';
	import {
		Mic,
		Globe,
		UnlockIcon,
		ShieldCheckIcon,
		GaugeIcon,
		RabbitIcon,
		LaptopMinimalCheckIcon,
		DownloadIcon,
		ServerIcon,
	} from '@lucide/svelte';
	import * as Card from '@eurora/ui/components/card/index';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import JoinWaitlist from './join_waitlist.svelte';

	import { SiYoutube } from '@icons-pack/svelte-simple-icons';

	// Animation state tracking
	let typingComplete = false;
	let showCursor = false;

	// Typing animation configuration
	const instantTyping = false;
	const firstPart = 'Explain ';
	const secondPart = 'this';
	const typingSpeed = instantTyping ? 0 : 150; // milliseconds per character
	const initialDelay = instantTyping ? 0 : 50; // milliseconds before typing starts
	const cardStaggerDelay = instantTyping ? 0 : 200; // milliseconds between cards (increased for better visual effect)

	let inputValue = '';
	let purpleText = '';
	let showTagLine = false;
	let taglineComponent: HTMLElement | null = null;

	// Simplified cards array without position data
	const totalCards = 3;
	const cards = Array.from({ length: totalCards }, (_, i) => ({
		id: i + 1,
		visible: false,
		animationDelay: i * cardStaggerDelay + 'ms',
	}));

	let visibleCards: number[] = [];

	// Function to start the card sequence
	function startCardSequence() {
		// Show cursor and keep it visible forever
		// showCursor = true;

		// Start showing cards with staggered animation
		let delay = 300; // Initial delay after typing completes

		cards.forEach((card, index) => {
			setTimeout(
				() => {
					visibleCards = [...visibleCards, card.id];
				},
				delay + index * cardStaggerDelay,
			);
		});

		// Show tagline after all cards are visible
		setTimeout(
			() => {
				showTagLine = true;
			},
			delay + cards.length * cardStaggerDelay + 400,
		);
	}

	onMount(() => {
		setTimeout(() => {
			// Type the first part normally
			let currentIndex = 0;
			const firstTypingInterval = setInterval(() => {
				inputValue += firstPart[currentIndex];
				currentIndex++;

				if (currentIndex === firstPart.length) {
					clearInterval(firstTypingInterval);

					// Start typing the second part in purple
					currentIndex = 0;
					const secondTypingInterval = setInterval(() => {
						purpleText += secondPart[currentIndex];
						currentIndex++;

						if (currentIndex === secondPart.length) {
							clearInterval(secondTypingInterval);
							typingComplete = true;
							startCardSequence();
						}
					}, typingSpeed);
				}
			}, typingSpeed);
		}, initialDelay);
	});
</script>

<!-- <img
	src="/backgrounds/gradient.svg"
	alt="Background gradient for hero section"
	class="h-screen w-full absolute top-0 left-0 z-0"
	loading="eager"
	decoding="async"
/> -->
<!-- Main container with 4 equal rows -->
<div class="h-screen flex flex-col max-w-[100%] mx-auto">
	<!-- Row 1: Header (hidden on mobile, 25% on desktop) -->
	<div class="hidden md:flex flex-1 items-center justify-center px-4">
		<h1 class="w-full mx-auto text-3xl sm:text-4xl md:text-5xl font-bold text-center z-10">
			Your Open Source AI Assistant
		</h1>
	</div>

	<!-- Row 2: Input Box (33% on mobile, 25% on desktop) -->
	<div class="flex-1 flex justify-center px-4">
		<!-- Make self position to the top -->
		<div class="w-full max-w-2xl self-center md:self-start">
			<div class="animate-grow relative">
				<div
					class="flex w-full min-h-[80px] sm:min-h-[100px] items-center text-2xl sm:text-3xl md:text-4xl font-semibold rounded-2xl border border-gray-300 px-3 py-4 shadow-lg md:px-4 md:py-6 backdrop-blur-2xl bg-white/20"
				>
					<div class="flex-grow">
						<span class="text-black/80">{inputValue}</span>
						<span class="text-black/80">{purpleText}</span>
						{#if showCursor}
							<span class="cursor-blink">|</span>
						{/if}
					</div>
					<Mic class="text-black/80" size={32} />
				</div>
			</div>
		</div>
	</div>

	<!-- Row 3: Feature Cards (25% of screen height) -->
	<div class="flex-1 flex items-center justify-center px-4">
		<div class="w-full h-full max-w-6xl">
			<!-- Mobile: Vertical stacked cards (smaller) -->
			<div class="md:hidden">
				<div class="grid grid-cols-1 gap-3 max-w-sm mx-auto">
					{#if visibleCards.includes(1)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: 0ms;"
						>
							<Card.Root class="card-content h-20 bg-white/20">
								<Card.Content class="flex h-full items-center justify-center p-3">
									<div
										class="icon-animation flex items-center justify-center mr-3"
									>
										<GaugeIcon size={24} />
									</div>
									<div class="flex flex-col text-center">
										<Card.Title
											class="title-animation text-black/80 text-sm font-semibold"
											>Context Aware</Card.Title
										>
										<Card.Description class="text-black/80 text-xs"
											>17x faster prompts</Card.Description
										>
									</div>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}

					{#if visibleCards.includes(2)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: {cards[1].animationDelay};"
						>
							<Card.Root class="card-content h-20 bg-white/20">
								<Card.Content class="flex h-full items-center justify-center p-3">
									<div
										class="icon-animation flex items-center justify-center mr-3"
									>
										<ShieldCheckIcon size={24} />
									</div>
									<div class="flex flex-col text-center">
										<Card.Title
											class="title-animation text-black/80 text-sm font-semibold"
											>Secure & Private</Card.Title
										>
										<Card.Description class="text-black/80 text-xs"
											>End-to-end encrypted</Card.Description
										>
									</div>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}

					{#if visibleCards.includes(3)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: {cards[2].animationDelay};"
						>
							<Card.Root class="card-content h-20 bg-white/20">
								<Card.Content class="flex h-full items-center justify-center p-3">
									<div
										class="icon-animation flex items-center justify-center mr-3"
									>
										<ServerIcon size={24} />
									</div>
									<div class="flex flex-col text-center">
										<Card.Title
											class="title-animation text-black/80 text-sm font-semibold"
											>Run Locally</Card.Title
										>
										<Card.Description class="text-black/80 text-xs"
											>Free forever</Card.Description
										>
									</div>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}
				</div>
			</div>

			<!-- Desktop: Grid layout -->
			<div class="hidden md:block">
				<div class=" md:grid grid-cols-3 gap-4">
					{#if visibleCards.includes(1)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: 0ms;"
						>
							<Card.Root class="card-content h-full bg-white/20">
								<Card.Content
									class="flex h-full flex-col items-center justify-center p-4"
								>
									<div class="icon-animation flex items-center justify-center">
										<GaugeIcon size={48} />
									</div>
									<Card.Title
										class="title-animation text-black/80 text-center text-lg sm:text-xl mt-2"
										>Context Aware</Card.Title
									>
									<Card.Description
										class="text-black/80 mt-1 text-sm sm:text-base text-center"
										>Prompt up to 17x faster</Card.Description
									>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}

					{#if visibleCards.includes(2)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: {cards[1].animationDelay};"
						>
							<Card.Root class="card-content h-full bg-white/20">
								<Card.Content
									class="flex h-full flex-col items-center justify-center p-4"
								>
									<div class="icon-animation flex items-center justify-center">
										<ShieldCheckIcon size={48} />
									</div>
									<Card.Title
										class="title-animation text-black/80 text-center text-lg sm:text-xl mt-2"
										>Secure and Private</Card.Title
									>
									<Card.Description
										class="text-black/80 mt-1 text-sm sm:text-base text-center"
										>End-to-end encryption</Card.Description
									>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}

					{#if visibleCards.includes(3)}
						<div
							class="card-entrance backdrop-blur-2xl"
							style="--animation-delay: {cards[2].animationDelay};"
						>
							<Card.Root class="card-content h-full bg-white/20">
								<Card.Content
									class="flex h-full flex-col items-center justify-center p-4"
								>
									<div class="icon-animation flex items-center justify-center">
										<ServerIcon size={48} />
									</div>
									<Card.Title
										class="title-animation text-black/80 text-center text-lg sm:text-xl mt-2"
										>Run Locally</Card.Title
									>
									<Card.Description
										class="text-black/80 mt-1 text-sm sm:text-base text-center"
										>For free, forever</Card.Description
									>
								</Card.Content>
							</Card.Root>
						</div>
					{/if}
				</div>
			</div>
		</div>
	</div>

	<!-- Row 4: Tagline and CTA (25% of screen height) -->
	<div class="flex-1 flex items-center justify-center px-4">
		{#if showTagLine}
			<div class="tagline-entrance text-center w-full max-w-4xl" bind:this={taglineComponent}>
				<h1 class="fade-in-up mb-4 font-bold text-2xl sm:text-3xl md:text-4xl">
					AI On Your Own Terms
				</h1>
				<div class="fade-in-up" style="--animation-delay: 200ms;">
					<Button
						class="mt-4 px-4 py-2 sm:px-6 sm:py-3"
						variant="default"
						onclick={(e) => {
							const taglineRect = taglineComponent?.getBoundingClientRect() ?? {
								top: 0,
							};
							window.scrollTo({
								top: window.scrollY + taglineRect.top + 200,
								behavior: 'smooth',
							});
						}}
					>
						Learn More
					</Button>
				</div>
			</div>
		{/if}
	</div>
</div>

<style lang="postcss">
	/* Mobile-first responsive design */
	@media (max-width: 768px) {
		.card-entrance {
			/* Adjust animation timing for mobile */
			animation-duration: 0.5s;
		}

		/* Ensure cards have proper spacing on mobile */
		:global(.card-content) {
			margin-bottom: 0.5rem;
		}

		/* Adjust icon size for mobile */
		:global(.icon-animation svg) {
			transform: scale(0.8);
		}

		/* Adjust text sizes for mobile */
		h1 {
			font-size: 1.5rem !important;
		}
	}

	/* Ensure proper height distribution for flex layout */
	.flex-1 {
		min-height: 0;
	}

	/* Horizontal scroll styling for mobile cards */
	.overflow-x-auto {
		scrollbar-width: none; /* Firefox */
		-ms-overflow-style: none; /* Internet Explorer 10+ */
	}

	.overflow-x-auto::-webkit-scrollbar {
		display: none; /* WebKit */
	}

	/* Smooth scroll snapping */
	.snap-x {
		scroll-snap-type: x mandatory;
	}

	.snap-center {
		scroll-snap-align: center;
	}

	/* Cursor blinking animation */
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
	}

	.cursor-blink {
		display: inline-block;
		margin-left: 2px;
		color: #9333ea; /* Purple color to match the theme */
		font-weight: 300;
		animation: blink 1.5s infinite;
	}

	/* Input box grow animation */
	@keyframes grow {
		from {
			transform: scale(0.2);
		}
		to {
			transform: scale(1);
		}
	}

	.animate-grow {
		animation: grow var(--animation-duration) cubic-bezier(0.34, 1.56, 0.64, 1);
	}

	:global(.animate-grow) {
		--animation-duration: 400ms;
	}

	/* Card entrance animation with staggered delay */
	.card-entrance {
		transform: translateY(30px) scale(0.95);
		animation: slideIn 0.6s cubic-bezier(0.22, 1, 0.36, 1) forwards;
		animation-delay: var(--animation-delay, 0ms);
		opacity: 0;
	}

	@keyframes slideIn {
		0% {
			transform: translateY(30px) scale(0.95);
			opacity: 0;
		}
		100% {
			transform: translateY(0) scale(1);
			opacity: 1;
		}
	}

	/* Card content animations */
	.card-content {
		box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.05);
		transition: all 0.3s ease;
	}

	.card-content:hover {
		transform: translateY(-5px);
		box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1);
	}

	.icon-animation {
		transform: scale(0.8);
		animation: fadeScale 0.5s ease-out forwards;
		animation-delay: calc(var(--animation-delay, 0ms) + 100ms);
		opacity: 0;
	}

	.title-animation {
		transform: translateY(10px);
		animation: fadeUp 0.5s ease-out forwards;
		animation-delay: calc(var(--animation-delay, 0ms) + 200ms);
		opacity: 0;
	}

	@keyframes fadeScale {
		to {
			transform: scale(1);
			opacity: 1;
		}
	}

	@keyframes fadeUp {
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}

	/* Tagline animations */
	.tagline-entrance {
		animation: fadeIn 0.8s ease-out forwards;
		opacity: 0;
	}

	.fade-in-up {
		transform: translateY(20px);
		animation: fadeInUp 0.7s cubic-bezier(0.22, 1, 0.36, 1) forwards;
		animation-delay: var(--animation-delay, 0ms);
		opacity: 0;
	}

	@keyframes fadeIn {
		to {
			opacity: 1;
		}
	}

	@keyframes fadeInUp {
		to {
			transform: translateY(0);
			opacity: 1;
		}
	}
</style>
