<script lang="ts">
	import { DownloadService, type OSType } from '$lib/download/downloadService';
	import { getOS, getOSDisplayName } from '$lib/utils/getOS';
	import { Button } from '@eurora/ui/components/button/index';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import LoaderIcon from '@lucide/svelte/icons/loader';
	import { onMount } from 'svelte';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();

	let os = $state<OSType>('unknown');
	let isLoading = $state(false);
	let error = $state<string | null>(null);

	const downloadService = new DownloadService();

	onMount(() => {
		os = getOS();
	});

	async function handleDownload() {
		if (os === 'unknown') {
			// Redirect to download page for manual selection
			// window.location.href = '/download';
			return;
		}

		isLoading = true;
		error = null;

		try {
			const success = await downloadService.initiateDownload(os);
			if (!success) {
				error = 'Download not available for your platform';
				// Redirect to download page after a delay
				setTimeout(() => {
					// window.location.href = '/download';
				}, 2000);
			}
		} catch (e) {
			error = 'Failed to start download';
			console.error('Download error:', e);
		} finally {
			isLoading = false;
		}
	}
</script>

<Button
	size="lg"
	class="md:w-auto p-8 shadow-lg gap-4 {className}"
	onclick={handleDownload}
	disabled={isLoading}
>
	{#if isLoading}
		<LoaderIcon size={32} class="animate-spin" />
		Starting download...
	{:else}
		<DownloadIcon size={32} />
		Download for {getOSDisplayName(os)}
	{/if}
</Button>

{#if error}
	<p class="text-red-500 text-sm mt-2">{error}</p>
{/if}
