<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import {
		getDownloadOptions,
		getDownloadUrl,
		type DownloadOption,
	} from '$lib/download/downloadService';
	import { getArch, getOS, getOSDisplayName } from '$lib/utils/getOS';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Collapsible from '@eurora/ui/components/collapsible/index';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import LaptopIcon from '@lucide/svelte/icons/laptop';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import type { Component } from 'svelte';

	let detectedOS = $state(getOS());
	let detectedArch = $state(getArch());
	let downloadOptions = $derived(getDownloadOptions(detectedOS, detectedArch));
	let allOptions = $derived(getDownloadOptions('unknown', 'unknown'));

	let platformDetected = $derived(detectedOS !== 'unknown');
	let singleOption = $derived(downloadOptions.length === 1);

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

<div class="mx-auto flex max-w-2xl flex-col items-center px-4 py-24 gap-8">
	<h1 class="mb-3 text-4xl font-bold tracking-tight text-foreground">Get Eurora</h1>

	{#if singleOption}
		<DownloadButton class="size-14 text-2xl" iconClass="w-6 h-6 size-6" />
	{:else if platformDetected}
		{@const osName = getOSDisplayName(detectedOS)}
		<p class="mb-4 text-sm text-muted-foreground">
			Choose your {osName} architecture
		</p>
		<div class="flex flex-wrap justify-center gap-3">
			{#each downloadOptions as option}
				<Button
					size="lg"
					class="h-14 rounded-xl px-8 text-lg shadow-md"
					onclick={() => handleDownload(option)}
				>
					<DownloadIcon class="size-5" />
					{option.label} ({option.archLabel})
				</Button>
			{/each}
		</div>
	{:else}
		<div class="grid w-full gap-3 sm:grid-cols-2">
			{#each downloadOptions as option}
				{@const Icon = getOSIcon(option.os)}
				<Button
					class="flex items-center gap-4 rounded-xl border border-border bg-card p-5 text-left transition-colors hover:bg-accent"
					onclick={() => handleDownload(option)}
				>
					<div class="rounded-lg bg-primary/10 p-3">
						<Icon class="size-5 text-primary" />
					</div>
					<div class="flex-1">
						<div class="font-medium text-foreground">{option.label}</div>
						<div class="text-sm text-muted-foreground">{option.archLabel}</div>
					</div>
					<DownloadIcon class="size-4 text-muted-foreground" />
				</Button>
			{/each}
		</div>
	{/if}
	<Button href="download/browser-extension/redirect">Download Browser Extension</Button>

	{#if platformDetected}
		<Collapsible.Root class="mt-8 w-full" open={true}>
			<Collapsible.Trigger
				class="mx-auto flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground [&[data-state=open]>svg]:rotate-180"
			>
				Other platforms
				<ChevronDownIcon class="size-4 transition-transform" />
			</Collapsible.Trigger>
			<Collapsible.Content>
				<div class="mt-4 grid w-full gap-2 sm:grid-cols-2">
					{#each allOptions as option}
						{@const Icon = getOSIcon(option.os)}
						<Button
							class="flex items-center gap-3 rounded-lg border border-border p-3 text-left transition-colors hover:bg-accent"
							onclick={() => handleDownload(option)}
							variant="outline"
						>
							<Icon class="size-4 text-muted-foreground" />
							<span class="text-sm font-medium text-foreground">{option.label}</span>
							<span class="text-xs text-muted-foreground">({option.archLabel})</span>
							<DownloadIcon class="ml-auto size-3.5 text-muted-foreground" />
						</Button>
					{/each}
				</div>
			</Collapsible.Content>
		</Collapsible.Root>
	{/if}
</div>
