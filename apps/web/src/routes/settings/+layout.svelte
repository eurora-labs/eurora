<script lang="ts">
	import { page } from '$app/state';
	import MenuBar from '$lib/components/MenuBar.svelte';
	import { currentUser, isAuthenticated } from '$lib/stores/auth.js';
	import {
		subscriptionStore,
		subscription,
		subscriptionLoading,
	} from '$lib/stores/subscription.js';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { ContactDialog } from '@eurora/ui/custom-components/contact-dialog/index';
	import ExternalLinkIcon from '@lucide/svelte/icons/external-link';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import SquarePen from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';

	let contactDialogOpen = $state(false);

	const STRIPE_PRO_PRICE_ID = import.meta.env.VITE_STRIPE_PRO_PRICE_ID;

	let { children } = $props();

	const planLabel = $derived(
		$subscription?.subscription_id && $subscription?.status === 'active'
			? $subscription.price_id === STRIPE_PRO_PRICE_ID
				? 'Pro'
				: 'Pro'
			: 'Free',
	);

	const navItems = [
		{ title: 'General', url: '/settings' },
		{ title: 'Billing', url: '/settings/billing' },
	];

	let items = $derived(
		navItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);

	onMount(() => {
		if (!$isAuthenticated) return;
		subscriptionStore.fetch();
	});
</script>

<div class="flex min-h-screen flex-col">
	<MenuBar />

	<div class="flex flex-1 flex-col pt-16">
		<div class="mx-auto flex w-full max-w-7xl items-start gap-12 px-8 py-10">
			<nav class="flex w-56 shrink-0 flex-col gap-0.5">
				{#if $currentUser}
					<div class="mb-4 flex flex-col overflow-hidden py-1 px-2">
						{#if $subscriptionLoading}
							<div class="flex items-center gap-2 py-0.5">
								<Loader2Icon size={14} class="animate-spin text-muted-foreground" />
								<span class="text-xs text-muted-foreground">Loading…</span>
							</div>
						{:else}
							<span class="flex items-center gap-1.5 leading-tight">
								<span class="truncate text-sm font-medium"
									>{$currentUser.name || 'User'}</span
								>
								<a
									href="/settings"
									class="shrink-0 text-muted-foreground transition-colors hover:text-foreground"
								>
									<SquarePen size={11} />
								</a>
							</span>
							<span class="truncate text-xs leading-tight text-muted-foreground">
								{planLabel} - {$currentUser.email}
							</span>
						{/if}
					</div>
				{/if}

				{#each items as item (item.title)}
					<a
						href={item.url}
						class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors
							{item.isActive ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'}"
					>
						{item.title}
					</a>
				{/each}

				<Separator class="my-2" />

				<a
					href="/docs"
					target="_blank"
					rel="noopener noreferrer"
					class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
				>
					Docs
					<ExternalLinkIcon size={12} class="ml-auto text-muted-foreground" />
				</a>

				<button
					type="button"
					class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:text-foreground"
					onclick={() => (contactDialogOpen = true)}
				>
					Contact
				</button>

				<ContactDialog bind:open={contactDialogOpen} showWebsiteLink={false} />
			</nav>

			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
	</div>
</div>
