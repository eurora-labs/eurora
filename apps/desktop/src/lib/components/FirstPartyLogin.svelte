<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import * as Card from '@eurora/ui/components/card/index';
	import { open } from '@tauri-apps/plugin-shell';

	const taurpc = inject(TAURPC_SERVICE);
	async function openLogin() {
		const loginToken = await taurpc.auth.get_login_token();
		await open(loginToken.url);

		const interval = setInterval(async () => {
			const isLoginSuccess = await taurpc.auth.poll_for_login();
			if (!isLoginSuccess) {
				return;
			}
			goto('/');
			clearInterval(interval);
		}, 5000);
	}
</script>

<Card.Root class="flex group cursor-pointer w-1/2" onclick={openLogin}>
	<Card.Header class="pb-6 text-left ">
		<Card.Title class="mb-2 text-2xl font-semibold">Get started with Eurora</Card.Title>
		<Card.Description class="">Fastest way to get started.</Card.Description>
	</Card.Header>
</Card.Root>
