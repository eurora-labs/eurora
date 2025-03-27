<script lang="ts">
	// Removed Input import as we're using a custom div
	import { onMount } from 'svelte';
	import { Mic, ScrollText, Youtube, TvMinimalPlay, Globe } from 'lucide-svelte';
	import { Card, Button, Input } from '@eurora/ui';
	import { Sheet, ScrollArea, Skeleton } from '@eurora/ui';
	import { buttonVariants } from '@eurora/ui';
    import WaitlistForm from './waitlist_form.svelte';

    import { SiYoutube } from '@icons-pack/svelte-simple-icons';


	// Typing animation configuration
	const instantTyping = false;
	const firstPart = 'Explain ';
	const secondPart = 'this';
	const typingSpeed = instantTyping ? 0 : 150; // milliseconds per character
	const initialDelay = instantTyping ? 0 : 50; // milliseconds before typing starts
	const cardDelay = instantTyping ? 0 : 50; // milliseconds between cards

	const firstCardDelay = typingSpeed * (firstPart.length + secondPart.length) + typingSpeed;
	const totalCards = 3;
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

<div style="height: 100vh;">
	<div class="mx-auto px-4 pt-6" style="height: 55vh;">
		<div class="grid grid-cols-12 gap-6">
			<!-- Input box centered in the first row -->
			<div class="col-span-12 flex justify-center mb-4">
				<div class="w-3/4">
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
			</div>

			<!-- Three cards in a row below the input box -->
			<div class="col-span-12 grid grid-cols-12 gap-6">
				{#if visibleCards.includes(1)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content class="flex flex-col items-center justify-center h-full">
								<div class="flex justify-center items-center">
									<!-- <TvMinimalPlay size={128} color="purple" /> -->
                                     <SiYoutube color="rgb(147 51 234 / var(--tw-text-opacity, 1))" size={64}  />
								</div>
                                <Card.Title>YouTube Videos</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}
				
				{#if visibleCards.includes(2)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content class="flex flex-col items-center justify-center h-full">
								<div class="flex justify-center items-center">
									<ScrollText class="text-purple-600" size={64} />
								</div>
                                <Card.Title>PDF Documents</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}
				
				{#if visibleCards.includes(3)}
					<div class="col-span-4 card-entrance">
						<Card.Root class="w-full aspect-video">
							<Card.Content class="flex flex-col items-center justify-center h-full">
								<div class="flex justify-center items-center">
									<Globe class="text-purple-600" size={64} />
								</div>
                                <Card.Title>Any Other Websites</Card.Title>
							</Card.Content>
						</Card.Root>
					</div>
				{/if}
			</div>
		</div>
	</div>

	{#if showTagLine}
		<div class="text-center mt-24 card-entrance" bind:this={taglineComponent}>
			<Button
				class="px-6 py-3"
				onclick={(e) => {
					const taglineRect = taglineComponent?.getBoundingClientRect() ?? { top: 0 };
					// const buttonRect = (e.target as HTMLElement).getBoundingClientRect();
					window.scrollTo({
						top: window.scrollY + taglineRect.top + 150,
						behavior: 'smooth'
					});
				}}
			>
				Learn More
			</Button>
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
