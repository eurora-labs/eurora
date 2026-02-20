<script lang="ts">
	import GetProButton from '$lib/components/GetProButton.svelte';
	import {
		getDownloadOptions,
		getDownloadUrl,
		type DownloadOption,
	} from '$lib/download/downloadService';
	import { currentUser } from '$lib/stores/auth.js';
	import { subscription, subscriptionLoading } from '$lib/stores/subscription.js';
	import { getArch, getOS } from '$lib/utils/getOS';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import * as Collapsible from '@eurora/ui/components/collapsible/index';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import LaptopIcon from '@lucide/svelte/icons/laptop';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import type { Component } from 'svelte';

	const isFreePlan = $derived(!$subscriptionLoading && !$subscription?.subscription_id);

	let detectedOS = $state(getOS());
	let detectedArch = $state(getArch());
	let downloadOptions = $derived(getDownloadOptions(detectedOS, detectedArch));
	let allOptions = $derived(getDownloadOptions('unknown', 'unknown'));
	let platformDetected = $derived(detectedOS !== 'unknown');

	function handleDownload(option: DownloadOption) {
		window.location.href = getDownloadUrl(option);
	}

	function getOSIcon(os: string): Component<{ class?: string }> {
		switch (os) {
			case 'macos':
				return LaptopIcon;
			default:
				return MonitorIcon;
		}
	}

	$effect(() => {
		detectedOS = getOS();
		detectedArch = getArch();
	});
</script>

<svelte:head>
	<title>{$currentUser?.name || 'User'} - Eurora Labs</title>
</svelte:head>

{#if $currentUser}
	{#if isFreePlan}
		<Card.Root
			class="relative overflow-hidden bg-linear-to-r from-primary/5 via-primary/10 to-transparent p-6"
		>
			<div class="flex items-center justify-between gap-4">
				<div class="flex items-start gap-4">
					<div
						class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary/10"
					>
						<SparklesIcon size={20} class="text-primary" />
					</div>
					<div>
						<h3 class="text-base font-semibold">Upgrade to Pro</h3>
						<p class="mt-1 text-sm text-muted-foreground">
							Unlock advanced features, priority support, and more with the Pro plan.
						</p>
					</div>
				</div>
				<GetProButton class="shrink-0">Upgrade</GetProButton>
			</div>
		</Card.Root>
	{/if}

	<div class="mt-6">
		<h3 class="mb-3 text-sm font-medium">Download Eurora</h3>
		{#if platformDetected}
			<div class="flex flex-wrap gap-2">
				{#each downloadOptions as option}
					<Button variant="outline" class="gap-2" onclick={() => handleDownload(option)}>
						{option.label} ({option.archLabel})
						<DownloadIcon size={16} />
					</Button>
				{/each}
				<Button variant="outline" class="gap-2" href="/download/browser-extension/redirect">
					<GlobeIcon size={16} />
					Browser Extension
				</Button>
			</div>
			<Collapsible.Root class="mt-4">
				<Collapsible.Trigger
					class="flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground [&[data-state=open]>svg]:rotate-180"
				>
					Other platforms
					<ChevronDownIcon class="size-4 transition-transform" />
				</Collapsible.Trigger>
				<Collapsible.Content>
					<div class="mt-2 flex flex-wrap gap-2">
						{#each allOptions as option}
							{@const Icon = getOSIcon(option.os)}
							<Button
								variant="outline"
								class="gap-2"
								onclick={() => handleDownload(option)}
							>
								<Icon class="size-4" />
								{option.label} ({option.archLabel})
								<DownloadIcon size={14} />
							</Button>
						{/each}
					</div>
				</Collapsible.Content>
			</Collapsible.Root>
		{:else}
			<div class="flex flex-wrap gap-2">
				{#each allOptions as option}
					{@const Icon = getOSIcon(option.os)}
					<Button variant="outline" class="gap-2" onclick={() => handleDownload(option)}>
						<Icon class="size-4" />
						{option.label} ({option.archLabel})
						<DownloadIcon size={14} />
					</Button>
				{/each}
				<Button variant="outline" class="gap-2" href="/download/browser-extension/redirect">
					<GlobeIcon size={16} />
					Browser Extension
				</Button>
			</div>
		{/if}
	</div>
{:else}
	<Card.Root class="p-6">
		<p class="text-muted-foreground text-sm">
			Please <a href="/login" class="underline">sign in</a> to view your profile.
		</p>
	</Card.Root>
{/if}
