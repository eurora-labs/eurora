<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let { children } = $props();

	initDependencies();

	const userService = inject(USER_SERVICE);
	userService.init().catch((err) => {
		console.error('Failed to initialize user service:', err);
		toast.error('Failed to initialize. Please restart the app.');
	});

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

<div class="app-shell relative flex flex-col h-dvh overflow-hidden">
	{@render children?.()}
</div>

<Toaster />
