<script lang="ts">
	import { goto } from '$app/navigation';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Card from '@eurora/ui/components/card/index';
	import { open } from '@tauri-apps/plugin-shell';

	const user = inject(USER_SERVICE);
	async function openLogin() {
		const loginToken = await user.getLoginToken();
		await open(loginToken.url);

		const interval = setInterval(async () => {
			const isLoginSuccess = await user.pollForLogin();
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
