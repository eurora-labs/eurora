<script lang="ts">
	// Removed Input import as we're using a custom div
	import { onMount } from 'svelte';
	import { Mic, ScrollText, Youtube, TvMinimalPlay, Globe } from '@lucide/svelte';
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

<div class="min-h-screen">
	<div class="mx-auto px-4 pt-6" style="min-height: 55vh;">
		<div class="grid grid-cols-12 gap-6">
			<!-- Input box centered in the first row -->
			<div class="col-span-12 mb-4 flex justify-center">
				<div class="w-full md:w-3/4">
					<div class="animate-grow relative">
						<div
							class="flex w-full items-center rounded-md border border-gray-300 bg-white px-3 py-4 shadow-lg md:px-4 md:py-6"
							style="font-size: clamp(28px, 5vw, 54px); min-height: 80px; height: auto;"
						>
							<div class="flex-grow">
								<span class="text-black">{inputValue}</span>
								<span class="text-purple-600">{purpleText}</span>
								{#if showCursor}
									<span class="cursor-blink">|</span>
								{/if}
							</div>
							<Mic class="h-8 w-8 text-gray-400" />
						</div>
					</div>
				</div>
			</div>

			<!-- Three cards in a row below the input box -->
			<div class="col-span-12 grid grid-cols-1 gap-6 md:grid-cols-12">
				{#if visibleCards.includes(1)}
					<div
						class="card-entrance col-span-12 mb-4 md:col-span-4 md:mb-0"
						style="--animation-delay: 0ms;"
					>
						<Card.Root class="card-content aspect-video w-full">
							<Card.Content class="flex h-full flex-col items-center justify-center">
								<div class="icon-animation flex items-center justify-center">
									<SiYoutube
										color="rgb(147 51 234 / var(--tw-text-opacity, 1))"
										size={64}
									/>
								</div>
								<Card.Title class="title-animation">YouTube Videos</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}

				{#if visibleCards.includes(2)}
					<div
						class="card-entrance col-span-12 mb-4 md:col-span-4 md:mb-0"
						style="--animation-delay: {cards[1].animationDelay};"
					>
						<Card.Root class="card-content aspect-video w-full">
							<Card.Content class="flex h-full flex-col items-center justify-center">
								<div class="icon-animation flex items-center justify-center">
									<ScrollText class="text-purple-600" size={64} />
								</div>
								<Card.Title class="title-animation">PDF Documents</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}

				{#if visibleCards.includes(3)}
					<div
						class="card-entrance col-span-12 md:col-span-4"
						style="--animation-delay: {cards[2].animationDelay};"
					>
						<Card.Root class="card-content aspect-video w-full">
							<Card.Content class="flex h-full flex-col items-center justify-center">
								<div class="icon-animation flex items-center justify-center">
									<Globe class="text-purple-600" size={64} />
								</div>
								<Card.Title class="title-animation">Any Other Websites</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#if showTagLine}
		<div
			class="tagline-entrance mt-12 hidden px-4 text-center md:mt-24 md:block"
			bind:this={taglineComponent}
		>
			<h1 class="fade-in-up mb-4 text-3xl font-bold md:text-4xl">AI On Your Own Terms</h1>
			<div class="fade-in-up" style="--animation-delay: 200ms;">
				<JoinWaitlist />
			</div>
			<!-- <Sheet.Root>
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
              </Sheet.Root> -->
			<div class="fade-in-up hidden md:block" style="--animation-delay: 400ms;">
				<Button
					class="mt-4 w-full px-4 py-2 sm:w-auto md:px-6 md:py-3"
					variant="outline"
					onclick={(e) => {
						const taglineRect = taglineComponent?.getBoundingClientRect() ?? { top: 0 };
						window.scrollTo({
							top: window.scrollY + taglineRect.top + 100,
							behavior: 'smooth',
						});
					}}
				>
					Learn More
				</Button>
			</div>
		</div>
		<!-- <div class="text-center" bind:this={taglineComponent}>
            <h1 class="text-5xl font-bold mb-6">Intelligence Without Compromise</h1>
            <p class="text-xl text-gray-600 max-w-3xl mx-auto mb-8">
                Eurora is a fully Open Source AI assistant that understands context, respects your privacy, and works across
                all your devices. Experience AI on your own terms.
            </p>
            <div class="flex justify-center gap-4">
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

                  <Button
				class="px-6 py-3"
                variant="outline"
				onclick={(e) => {
					const taglineRect = taglineComponent?.getBoundingClientRect() ?? { top: 0 };
					// const buttonRect = (e.target as HTMLElement).getBoundingClientRect();
					window.scrollTo({
						top: window.scrollY + taglineRect.top - 100,
						behavior: 'smooth'
					});
				}}
			>
				Learn More
			</Button>
                
            </div>
        </div> -->
	{/if}
</div>

<style>
	/* Mobile-first responsive design */
	@media (max-width: 768px) {
		.card-entrance {
			/* Adjust animation timing for mobile */
			animation-duration: 0.5s;
		}

		/* Ensure cards have proper spacing on mobile */
		:global(.card-content) {
			margin-bottom: 1rem;
		}

		/* Adjust icon size for mobile */
		:global(.icon-animation svg) {
			transform: scale(0.9);
		}
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
