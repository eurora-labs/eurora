<script lang="ts">
	import '$styles/styles.css';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		setMode('dark');

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

<div
	class="app-shell relative flex flex-col h-screen overflow-hidden"
	style="padding-top: var(--safe-area-top); padding-bottom: var(--safe-area-bottom);"
>
	<main class="flex-1 min-h-0 bg-background">
		{@render children?.()}
	</main>
</div>

<Toaster />
