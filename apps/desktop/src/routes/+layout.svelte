<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import AccessibilityPermission from '$lib/components/AccessibilityPermission.svelte';
	import ResizeHandles from '$lib/components/ResizeHandles.svelte';
	import Titlebar from '$lib/components/Titlebar.svelte';
	import UpdateChecker from '$lib/components/UpdateChecker.svelte';
	import { APPEARANCE_SERVICE } from '$lib/services/appearance-service.svelte.js';
	import { GENERAL_SERVICE } from '$lib/services/general-service.svelte.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { openUrl } from '@tauri-apps/plugin-opener';
	import { platform } from '@tauri-apps/plugin-os';
	import { ModeWatcher } from 'mode-watcher';
	import { onMount } from 'svelte';

	// Set platform class synchronously (before first render) so CSS
	// variables like --titlebar-height are correct from the start.
	const currentPlatform = platform();
	document.body.classList.add(`${currentPlatform}-app`);

	let { children } = $props();

	initDependencies();

	const userService = inject(USER_SERVICE);
	userService.init();

	const appearanceService = inject(APPEARANCE_SERVICE);
	appearanceService.init();

	const generalService = inject(GENERAL_SERVICE);
	generalService.init();

	const timelineService = inject(TIMELINE_SERVICE);
	timelineService.init();

	onMount(() => {
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

<ModeWatcher defaultMode="system" />

<div class="app-shell flex flex-col overflow-hidden bg-background">
	<Titlebar />
	<main class="flex-1 min-h-0 bg-background">
		{@render children?.()}
	</main>
</div>

<AccessibilityPermission />
<UpdateChecker />
<Toaster />

<!--
	Resize handles cover the OS window rect (the full viewport, including
	the Linux shadow gutter), so they are siblings of the visually-rounded
	shell rather than children clipped by it. macOS uses native edge resize.
-->
{#if currentPlatform !== 'macos'}
	<ResizeHandles />
{/if}

<style>
	.app-shell {
		position: fixed;
		inset: 0;
		border-radius: var(--shell-radius);
	}
</style>
