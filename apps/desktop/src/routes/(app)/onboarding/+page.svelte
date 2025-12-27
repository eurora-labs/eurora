<script lang="ts">
	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
</script>

<div class="relative flex h-full w-full flex-col">
	<div class="relative z-10 flex h-full flex-col">
		<!-- Title in middle top -->
		<div class="flex justify-center pt-16 pb-0">
			<h1 class="text-4xl font-bold drop-shadow-lg">Welcome to Eurora!</h1>
		</div>

		<!-- Main content area with two cards side by side -->
		<div class="flex flex-1 items-center justify-center px-8">
			<div class="grid w-full max-w-4xl grid-cols-1 gap-8 md:grid-cols-2">
				<!-- Left side - Log in or Sign up card -->

				<FirstPartyLogin />

				<!-- Right side - Connect to local card -->
				<Card.Root
					class="group cursor-pointer border-white/20 backdrop-blur-md transition-all duration-300 hover:bg-white/15"
				>
					<Card.Header class="pb-6 text-center">
						<Card.Title class="mb-2 text-2xl font-semibold">Local Connection</Card.Title
						>
						<Card.Description class="">
							Connect to your local AI model for offline usage
						</Card.Description>
					</Card.Header>
					<Card.Content class="flex justify-center">
						<Button
							href="/onboarding/local"
							variant="secondary"
							class="w-full rounded-lg px-6 py-3 font-medium transition-colors duration-200"
							size="lg"
						>
							Connect to local
						</Button>
					</Card.Content>
				</Card.Root>
			</div>
		</div>

		<!-- Footer space -->
		<div class="pb-16"></div>
	</div>
</div>
