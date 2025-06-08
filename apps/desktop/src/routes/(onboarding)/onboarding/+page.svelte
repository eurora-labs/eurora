<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { open } from '@tauri-apps/plugin-shell';

	// import tauri auth procedures
	import { createTauRPCProxy } from '@eurora/tauri-bindings';
	const taurrpc = createTauRPCProxy();
	async function openLogin() {
		const loginToken = await taurrpc.auth.get_login_token();
		await open(loginToken.url);

		// Attempt to login by token every 5 seconds
		const interval = setInterval(async () => {
			// if (loginToken.expires_in < Date.now()) {
			// 	clearInterval(interval);
			// 	return;
			// }

			const isLoginSuccess = await taurrpc.auth.poll_for_login();
			if (!isLoginSuccess) {
				console.log('Login not successful');
				return;
			}
			console.log('Login successful');
			clearInterval(interval);
			// window.location.href = '/';
		}, 5000);
	}
</script>

<div class="relative flex h-screen w-full flex-col">
	<div class="relative z-10 flex h-full flex-col">
		<!-- Title in middle top -->
		<div class="flex justify-center pt-16 pb-0">
			<h1 class="text-4xl font-bold drop-shadow-lg">Welcome to Eurora!</h1>
		</div>

		<!-- Main content area with two cards side by side -->
		<div class="flex flex-1 items-center justify-center px-8">
			<div class="grid w-full max-w-4xl grid-cols-1 gap-8 md:grid-cols-2">
				<!-- Left side - Log in or Sign up card -->
				<Card.Root
					class="group cursor-pointer border-white/20 backdrop-blur-md transition-all duration-300 hover:bg-white/15"
				>
					<Card.Header class="pb-6 text-center">
						<Card.Title class="mb-2 text-2xl font-semibold"
							>Log in or Sign up</Card.Title
						>
						<Card.Description class="">
							Sign in to your existing account or create a new one
						</Card.Description>
					</Card.Header>
					<Card.Content class="flex justify-center">
						<Button
							onclick={openLogin}
							variant="default"
							class="w-full rounded-lg px-6 py-3 font-medium transition-colors duration-200"
							size="lg"
						>
							Log in or Sign Up
						</Button>
					</Card.Content>
				</Card.Root>

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
							href="/onboarding/llama"
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
