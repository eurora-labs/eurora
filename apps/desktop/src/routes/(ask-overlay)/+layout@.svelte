<script lang="ts">
	import '$styles/styles.css';
	import { initDependencies } from '$lib/bootstrap/deps.js';
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { APPEARANCE_SERVICE } from '$lib/services/appearance-service.svelte.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Toaster } from '@eurora/ui/components/sonner/index';
	import { platform } from '@tauri-apps/plugin-os';
	import { ModeWatcher } from 'mode-watcher';

	// Platform class drives a handful of OS-conditional CSS rules
	// (drag region cursor on Linux, blur intensity on macOS). The
	// overlay shells set it the same way the main shell does.
	const currentPlatform = platform();
	document.body.classList.add(`${currentPlatform}-app`);
	document.body.classList.add('ask-overlay-body');

	let { children } = $props();

	// Initialize the same dependency graph the main window uses. The
	// overlay needs ChatService (for streaming answers), TimelineService
	// (for the focused-app icon), ActivityService (the chat context
	// provider), and UserService (auth gating). Re-initializing in this
	// webview is intentional: each Tauri webview is a separate JS
	// context, so the main window's services aren't reachable here.
	initDependencies();

	const appearance = inject(APPEARANCE_SERVICE);
	const timeline = inject(TIMELINE_SERVICE);
	const activity = inject(ACTIVITY_SERVICE);
	const user = inject(USER_SERVICE);

	appearance.init();
	timeline.init();
	activity.init();
	user.init();

	// Tauri broadcast channels don't replay history, so this webview's
	// `recent[]` is empty at mount even though the main webview has
	// been collecting events for a while. Pull the current focused-app
	// snapshot once so the icon appears immediately — without this,
	// the icon stays blank until the next OS focus change.
	timeline.seedFromCurrentActivity();
</script>

<ModeWatcher defaultMode="system" />
<div class="ask-shell">
	{@render children?.()}
</div>
<Toaster />

<style>
	:global(html, body) {
		background: transparent !important;
	}
	:global(.ask-overlay-body) {
		overflow: hidden;
		background: transparent !important;
		color: var(--foreground);
	}
	.ask-shell {
		display: flex;
		position: fixed;
		flex-direction: column;
		inset: 0;
		overflow: hidden;
		border: 1px solid color-mix(in oklab, var(--foreground) 12%, transparent);
		border-radius: 14px;
		backdrop-filter: blur(28px) saturate(140%);
		-webkit-backdrop-filter: blur(28px) saturate(140%);
		background: color-mix(in oklab, var(--background) 60%, transparent);
		box-shadow: 0 24px 64px rgba(0, 0, 0, 0.35);
	}
</style>
