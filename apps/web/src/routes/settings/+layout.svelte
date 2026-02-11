<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { auth, currentUser, accessToken, isAuthenticated } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import MailIcon from '@lucide/svelte/icons/mail';
	import PenLineIcon from '@lucide/svelte/icons/pen-line';

	const PAYMENT_API_URL = import.meta.env.VITE_PAYMENT_API_URL;
	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID;

	let { children } = $props();
	let planLabel = $state('Free');

	const navItems = [
		{ title: 'General', url: '/settings', icon: BoltIcon },
		{ title: 'Billing', url: '/settings/billing', icon: CreditCardIcon },
		{ title: 'Docs', url: '/settings/documentation', icon: BookOpenIcon },
	];

	let items = $derived(
		navItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);

	onMount(async () => {
		if (!$isAuthenticated) return;
		try {
			await auth.ensureValidToken();
			const res = await fetch(`${PAYMENT_API_URL}/payment/subscription`, {
				headers: { Authorization: `Bearer ${$accessToken}` },
			});
			if (res.ok) {
				const data = await res.json();
				if (data.subscription_id && data.status === 'active') {
					planLabel = data.price_id === STRIPE_PRO_PRICE_ID ? 'Pro' : 'Pro';
				}
			}
		} catch {
			// Silently fall back to "Free"
		}
	});
</script>

<div class="flex min-h-screen flex-col">
	<header class="backdrop-blur">
		<div class="flex h-14 w-full items-center px-6">
			<Button
				variant="link"
				class="decoration-transparent gap-2.5 p-0 font-semibold"
				href="/"
			>
				<EuroraLogo style="width: 2rem; height: 2rem;" />
			</Button>
		</div>
	</header>

	<div class="flex flex-1 flex-col pt-16">
		<div class="mx-auto flex w-full max-w-5xl items-start gap-12 px-8 py-10">
			<nav class="flex w-56 shrink-0 flex-col gap-0.5">
				{#if $currentUser}
					<div class="mb-4 flex flex-col overflow-hidden py-1">
						<span class="flex items-center gap-1.5 leading-tight">
							<span class="truncate text-sm font-medium"
								>{$currentUser.name || 'User'}</span
							>
							<a
								href="/settings"
								class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
							>
								<PenLineIcon size={11} />
							</a>
						</span>
						<span class="truncate text-xs leading-tight text-muted-foreground">
							{planLabel} - {$currentUser.email}
						</span>
					</div>
				{/if}

				{#each items as item (item.title)}
					<a
						href={item.url}
						class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors
							{item.isActive ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'}"
					>
						<item.icon size={16} />
						{item.title}
					</a>
				{/each}

				<Dialog.Root>
					<Dialog.Trigger
						class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
					>
						<MailIcon size={16} />
						Contact
					</Dialog.Trigger>
					<Dialog.Content class="sm:max-w-1/2">
						<Dialog.Header class="items-start">
							<Dialog.Title>Contact Us</Dialog.Title>
						</Dialog.Header>
						<p>
							Feel free to reach out to us at
							<a href="mailto:contact@eurora-labs.com" class="inline w-fit underline"
								>contact@eurora-labs.com</a
							>
							for any inquiries or feedback.
						</p>
					</Dialog.Content>
				</Dialog.Root>
			</nav>

			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
	</div>
</div>
