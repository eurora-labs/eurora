<script lang="ts">
	import { getDownloadUrlForOS, type OSType, type ArchType } from '$lib/download/downloadService';
	import { getArch, getOS, getOSDisplayName } from '$lib/utils/getOS';
	import { Button } from '@eurora/ui/components/button/index';
	import DownloadIcon from '@lucide/svelte/icons/download';

	interface Props {
		class?: string;
		iconClass?: string;
	}

	let { class: className = '', iconClass = '' }: Props = $props();

	let os = $state<OSType>('unknown');
	let arch = $state<ArchType>('unknown');

	$effect(() => {
		os = getOS();
		arch = getArch();
	});

	function handleDownload() {
		if (os === 'unknown') {
			window.location.href = '/download';
			return;
		}

		window.location.href = getDownloadUrlForOS(os, arch);
	}
</script>

<Button size="lg" class="md:w-auto p-4 shadow-lg {className}" onclick={handleDownload}>
	Download for {getOSDisplayName(os)}
	<DownloadIcon class={iconClass} />
</Button>
