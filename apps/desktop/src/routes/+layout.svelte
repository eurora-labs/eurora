<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import AccessibilityPermission from '$lib/components/AccessibilityPermission.svelte';
	import Titlebar from '$lib/components/Titlebar.svelte';
	import UpdateChecker from '$lib/components/UpdateChecker.svelte';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { platform } from '@tauri-apps/plugin-os';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	// Set platform class synchronously (before first render) so CSS
	// variables like --titlebar-height are correct from the start.
	document.body.classList.add(`${platform()}-app`);

	let { children } = $props();

	initDependencies();

	onMount(() => {
		setMode('dark');

		// All urls open in a separate browser window
		document.addEventListener('click', handleUrls);

		return () => {
			document.removeEventListener('click', handleUrls);
		};
	});

	async function handleUrls(event: MouseEvent) {
		const target = event.target as HTMLElement | null;
		if (!target) return;

		const anchor = target.closest('a[href]') as HTMLAnchorElement | null;
		if (!anchor) return;

		const href = anchor.getAttribute('href');
		if (!href) return;

		// external http(s) links only
		const isExternal = /^https?:\/\//i.test(href);
		if (!isExternal) return;

		event.preventDefault();
		try {
			await openUrl(href);
		} catch (error) {
			console.error('Failed to open URL:', error);
		}
	}
</script>

<ModeWatcher defaultMode="dark" track={false} />

<div class="flex flex-col h-screen overflow-hidden">
	<Titlebar />
	<main class="flex-1 min-h-0 bg-inherit">
		{@render children?.()}
	</main>
</div>

<AccessibilityPermission />
<UpdateChecker />
<Toaster />
