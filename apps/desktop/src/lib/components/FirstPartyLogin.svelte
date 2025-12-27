<script lang="ts">
	import { goto } from '$app/navigation';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { open } from '@tauri-apps/plugin-shell';

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
				return;
			}
			goto('/');
			clearInterval(interval);
		}, 5000);
	}
</script>

<Card.Root
	class="group cursor-pointer border-white/20 backdrop-blur-md transition-all duration-300 hover:bg-white/15"
	onclick={openLogin}
>
	<Card.Header class="pb-6 text-center">
		<Card.Title class="mb-2 text-2xl font-semibold">Log in or Sign up</Card.Title>
		<Card.Description class="">
			Sign in to your existing account or create a new one
		</Card.Description>
	</Card.Header>
	<Card.Content class="flex justify-center">
		<Button
			variant="default"
			class="w-full rounded-lg px-6 py-3 font-medium transition-colors duration-200"
			size="lg"
		>
			Log in or Sign Up
		</Button>
	</Card.Content>
</Card.Root>
