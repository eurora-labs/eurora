<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { open } from '@tauri-apps/plugin-shell';
	import { onDestroy, onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);

	let interval: ReturnType<typeof setInterval> | undefined;

	function startPolling() {
		interval = setInterval(async () => {
			try {
				const subscribed = await taurpc.payment.is_subscribed();
				if (!subscribed) return;

				clearInterval(interval);
				await taurpc.auth.refresh_session().catch(() => {});
				const win = getCurrentWindow();
				await win.setFocus();
				goto('/');
			} catch (e) {
				console.warn('Upgrade poll error:', e);
			}
		}, 5000);
	}

	onMount(() => {
		startPolling();
	});

	onDestroy(() => {
		if (interval) clearInterval(interval);
	});
</script>

<div class="flex flex-col justify-center items-center h-full p-8">
	<div class="flex flex-col max-w-md gap-6">
		<div class="flex items-center gap-3">
			<Spinner class="w-6 h-6 shrink-0" />
			<h1 class="text-3xl font-bold">Completing your upgrade</h1>
		</div>

		<p class="text-muted-foreground">
			A checkout page has been opened in your browser. Complete the payment there and this
			page will automatically update once your subscription is active.
		</p>

		<Separator />

		<div class="flex flex-col gap-3">
			<p class="text-sm text-muted-foreground">
				If the page didn't open, click below to try again:
			</p>
			<Button
				variant="outline"
				class="w-fit"
				onclick={async () => {
					try {
						const url = await taurpc.payment.create_checkout_url();
						await open(url);
					} catch (e) {
						toast.error(`Failed to open checkout: ${e}`);
					}
				}}
			>
				Open checkout page
				<ExternalLink class="size-3.5" />
			</Button>
		</div>

		<Button variant="ghost" class="w-fit text-muted-foreground" onclick={() => goto('/')}>
			Cancel
		</Button>
	</div>
</div>
