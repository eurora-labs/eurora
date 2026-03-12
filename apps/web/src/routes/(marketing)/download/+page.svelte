<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import {
		getDownloadOptions,
		getDownloadUrl,
		type DownloadOption,
	} from '$lib/services/download-service';
	import { getArch, getOS } from '$lib/utils/getOS';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Collapsible from '@eurora/ui/components/collapsible/index';
	import AppleIcon from '@lucide/svelte/icons/apple';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import type { Component } from 'svelte';

	const detectedOS = getOS();
	const detectedArch = getArch();
	const downloadOptions = getDownloadOptions(detectedOS, detectedArch);
	const allOptions = getDownloadOptions('unknown', 'unknown');
	const platformDetected = detectedOS !== 'unknown';
	const alternatives = downloadOptions.slice(1);

	function handleDownload(option: DownloadOption) {
		window.location.href = getDownloadUrl(option);
	}

	function getOSIcon(os: string): Component<{ class?: string }> {
		switch (os) {
			case 'macos':
				return AppleIcon;
			default:
				return MonitorIcon;
		}
	}

	function groupByLabel(options: DownloadOption[]): Map<string, DownloadOption[]> {
		const map = new Map<string, DownloadOption[]>();
		for (const opt of options) {
			const key = opt.label;
			if (!map.has(key)) map.set(key, []);
			map.get(key)!.push(opt);
		}
		return map;
	}
</script>

<div class="mx-auto flex w-full max-w-5xl flex-col items-center gap-16 px-4 py-24">
	<section class="flex flex-col items-center gap-6 text-center">
		<p class="text-sm font-medium uppercase tracking-widest text-primary">Free & Open Source</p>
		<p class="max-w-xl text-lg text-muted-foreground sm:text-xl leading-relaxed">
			A private AI assistant that reads what you read. Available on every major platform.
		</p>
	</section>

	<section class="flex flex-col items-center gap-4">
		<DownloadButton class="h-20 w-md" />
		{#if platformDetected && alternatives.length > 0}
			<div
				class="flex flex-wrap justify-center items-center gap-2 text-sm text-muted-foreground"
			>
				<span>Other formats:</span>
				{#each alternatives as alt}
					<Button
						variant="ghost"
						size="sm"
						class="rounded-full border border-border"
						onclick={() => handleDownload(alt)}
					>
						<DownloadIcon class="size-3.5" />
						{alt.formatLabel ?? alt.archLabel}
					</Button>
				{/each}
			</div>
		{/if}
	</section>

	<Collapsible.Root class="flex w-full flex-col items-center" open={true}>
		<Collapsible.Trigger
			class="flex items-center gap-1 text-sm text-muted-foreground transition-colors hover:text-foreground [&[data-state=open]>svg]:rotate-180"
		>
			All platforms & architectures
			<ChevronDownIcon class="size-4 transition-transform" />
		</Collapsible.Trigger>
		<Collapsible.Content>
			<div class="mt-6 grid w-full gap-6 sm:grid-cols-3">
				{#each groupByLabel(allOptions) as [osName, options]}
					{@const Icon = getOSIcon(options[0].os)}
					<div
						class="group rounded-2xl border border-border bg-card/50 p-6 transition-colors hover:border-primary/20 hover:bg-card"
					>
						<div class="flex items-center gap-3 mb-4">
							<div class="rounded-xl bg-primary/10 p-2.5">
								<Icon class="size-5 text-primary" />
							</div>
							<h3 class="text-lg font-semibold">{osName}</h3>
						</div>
						<div class="flex flex-col gap-1">
							{#each options as option}
								<Button
									variant="ghost"
									class="flex w-full items-center justify-between rounded-lg px-3 py-2.5 text-left text-sm transition-colors hover:bg-accent"
									onclick={() => handleDownload(option)}
								>
									<div>
										<span class="font-medium text-foreground"
											>{option.archLabel}</span
										>
										{#if option.formatLabel}
											<span class="ml-1.5 text-muted-foreground"
												>{option.formatLabel}</span
											>
										{/if}
									</div>
									<DownloadIcon
										class="size-4 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
									/>
								</Button>
							{/each}
						</div>
					</div>
				{/each}
			</div>
		</Collapsible.Content>
	</Collapsible.Root>
</div>
