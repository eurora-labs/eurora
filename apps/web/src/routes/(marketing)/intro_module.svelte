<script lang="ts">
	// Removed Input import as we're using a custom div
	import { onMount } from 'svelte';
	import { Mic, ScrollText, Youtube, TvMinimalPlay } from 'lucide-svelte';
	import { Card, Button, Input } from '@eurora/ui';
	import { SiYoutube, SiApple } from '@icons-pack/svelte-simple-icons';

	// Typing animation configuration
	const instantTyping = true;
	const firstPart = 'Explain ';
	const secondPart = 'this';
	const typingSpeed = instantTyping ? 0 : 150; // milliseconds per character
	const initialDelay = instantTyping ? 0 : 50; // milliseconds before typing starts
	const cardDelay = instantTyping ? 0 : 50; // milliseconds between cards

	const firstCardDelay = typingSpeed * (firstPart.length + secondPart.length) + typingSpeed;
	const totalCards = 5;
	const tagLineDelay = instantTyping ? 0 : firstCardDelay + cardDelay * (totalCards - 1) + 50;

	let inputValue = '';
	let purpleText = '';
	let showTagLine = false;
	let taglineComponent: HTMLElement | null = null;

	// Simplified cards array without position data
	const cards = Array.from({ length: totalCards }, (_, i) => ({
		id: i + 1,
		delay: firstCardDelay + (i + 1) * cardDelay
	}));

	let visibleCards: number[] = [];

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
						}
					}, typingSpeed);
				}
			}, typingSpeed);
		}, initialDelay);

		// Add cards sequentially
		cards.forEach((card) => {
			setTimeout(() => {
				visibleCards = [...visibleCards, card.id];
			}, card.delay);
		});

		// Add tagline delay
		setTimeout(() => {
			showTagLine = true;
		}, tagLineDelay);
	});
</script>

<div class="h-screen">
	<div class="mx-auto px-4 pt-6" style="height: 75vh;">
		<div class="grid grid-cols-12 gap-6">
			<!-- First row with cards 1, 2 and input box -->
			<div class="col-span-12 grid grid-cols-12 gap-6 mb-6">
				{#if visibleCards.includes(1)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content class="flex flex-col items-center justify-center h-full">
								<!-- <div class="w-full h-10 bg-purple-600 rounded-md mb-4 max-w-xs mx-auto"></div> -->
								<div class="flex justify-center items-center">
									<!-- <SiYoutube size={128} color="purple" /> -->
									<TvMinimalPlay size={128} color="purple" />
									<!-- <TvMinimalPlay /> -->
									<!-- <ScrollText size={128} /> -->
								</div>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}

				<!-- Input box in the middle -->
				<div class="pt-16 col-span-4 {!visibleCards.length ? 'col-start-5' : ''}">
					<div class="relative animate-grow">
						<div
							class="py-6 px-4 shadow-lg rounded-md border border-gray-300 bg-white w-full flex items-center"
							style="font-size: 54px; height: 131px;"
						>
							<div class="flex-grow">
								<span class="text-black">{inputValue}</span>
								<span class="text-purple-600">{purpleText}</span>
							</div>
							<Mic class="h-8 w-8 text-gray-400" />
						</div>
					</div>
				</div>

				{#if visibleCards.includes(2)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content class="flex flex-col items-center justify-center h-full">
								<!-- <div class="w-full h-10 bg-purple-600 rounded-md mb-4 max-w-xs mx-auto"></div> -->
								<div class="flex justify-center items-center">
									<!-- <SiYoutube size={128} /> -->
									<ScrollText size={128} />
								</div>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}
			</div>

			<!-- Second row with cards 3, 4 and 5 -->
			<div class="col-span-12 grid grid-cols-12 gap-6 mb-6">
				{#if visibleCards.includes(3)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content>Desktop Screenshot 3</Card.Content>
						</Card.Root>
					</div>
				{/if}
				{#if visibleCards.includes(4)}
					<div class="col-span-4 col-start-5 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content>Desktop Screenshot 4</Card.Content>
						</Card.Root>
					</div>
				{/if}
				{#if visibleCards.includes(5)}
					<div class="col-span-4 col-start-9 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content>Desktop Screenshot 5</Card.Content>
						</Card.Root>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#if showTagLine}
		<div class="text-center mt-24 card-entrance" bind:this={taglineComponent}>
			<h1 class="text-4xl font-bold mb-4">AI for Humans</h1>
			<Button
				class="px-6 py-3"
				onclick={(e) => {
					const taglineRect = taglineComponent?.getBoundingClientRect() ?? { top: 0 };
					// const buttonRect = (e.target as HTMLElement).getBoundingClientRect();
					window.scrollTo({
						top: window.scrollY + taglineRect.top,
						behavior: 'smooth'
					});
				}}
			>
				Learn More
			</Button>
		</div>
	{/if}
</div>

<style>
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
		opacity: 0;
		transform: translateY(20px);
		animation: slideIn 0.5s ease-out forwards;
		/* Add transition for smoother repositioning if needed */
		transition: all 0.3s ease-out;
	}

	@keyframes slideIn {
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
