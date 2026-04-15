<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import MobileSidebar from '$lib/components/MobileSidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	let { children } = $props();

	initDependencies();

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
	<Sidebar.Provider>
		<MobileSidebar />
		<Sidebar.Inset>
			<header class="flex items-center gap-2 px-3 py-2 border-b border-border">
				<Sidebar.Trigger />
				<h1 class="text-sm font-semibold text-foreground">Eurora</h1>
			</header>
			<main class="flex-1 min-h-0 bg-background">
				{@render children?.()}
			</main>
		</Sidebar.Inset>
	</Sidebar.Provider>
</div>

<Toaster />
