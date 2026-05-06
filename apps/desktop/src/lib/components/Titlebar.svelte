<script lang="ts">
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { CHAT_SERVICE, type ViewMode } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Tabs from '@eurora/ui/components/tabs/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GitForkIcon from '@lucide/svelte/icons/git-fork';
	import ListIcon from '@lucide/svelte/icons/list';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import SquareIcon from '@lucide/svelte/icons/square';
	import XIcon from '@lucide/svelte/icons/x';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	const chatService = inject(CHAT_SERVICE);
	const user = inject(USER_SERVICE);

	const hasMessages = $derived((chatService.activeThread?.messages.length ?? 0) > 0);
	const activeThread = $derived(chatService.activeThread?.thread);
	const threadTitle = $derived(activeThread?.title ?? 'New Chat');
	const threadDateIso = $derived(activeThread?.created_at ?? new Date().toISOString());

	const dateFormatter = new Intl.DateTimeFormat('en', {
		month: 'short',
		day: 'numeric',
		year: 'numeric',
	});
	const threadDateLabel = $derived(dateFormatter.format(new Date(threadDateIso)));

	let maximized = $state(false);
	const currentPlatform = platform();
	const isMac = currentPlatform === 'macos';

	const appWindow = getCurrentWindow();

	onMount(() => {
		appWindow.isMaximized().then((val) => (maximized = val));

		const unlisten = appWindow.onResized(() => {
			appWindow.isMaximized().then((val) => (maximized = val));
		});

		return () => {
			unlisten.then((fn) => fn());
		};
	});

	function minimize() {
		appWindow.minimize();
	}

	function toggleMaximize() {
		appWindow.toggleMaximize();
	}

	function close() {
		appWindow.close();
	}
</script>

<!--
	Tauri's drag-region handler reads `data-tauri-drag-region` on `e.target`
	directly (no ancestor walk), so every non-interactive child of the bar
	carries the attribute. Interactive controls (Tabs, Min/Max/Close) omit
	it so they remain clickable.
-->
<div data-tauri-drag-region class="titlebar bg-background" class:titlebar-mac={isMac}>
	<div data-tauri-drag-region class="titlebar-fill"></div>
	<div data-tauri-drag-region class="titlebar-content">
		{#if hasMessages}
			<Tabs.Root
				value={chatService.viewMode}
				onValueChange={(v) => (chatService.viewMode = v as ViewMode)}
			>
				<Tabs.List class="h-7">
					<Tabs.Trigger value="list" class="h-5 gap-1 px-2 text-xs">
						<ListIcon size={12} />
						List
					</Tabs.Trigger>
					<Tabs.Trigger value="graph" class="h-5 gap-1 px-2 text-xs">
						<GitForkIcon size={12} />
						Graph
					</Tabs.Trigger>
				</Tabs.List>
			</Tabs.Root>
		{/if}
		<span
			data-tauri-drag-region
			class="truncate text-xs font-medium max-w-[28ch]"
			title={threadTitle}
		>
			{threadTitle}
		</span>
		<time
			data-tauri-drag-region
			class="text-xs text-muted-foreground whitespace-nowrap"
			datetime={threadDateIso}
		>
			{threadDateLabel}
		</time>
		{#if user.authenticated}
			<Badge
				data-tauri-drag-region
				variant={user.planLabel === 'Pro' ? 'outline' : 'secondary'}
				class="text-xs px-2 py-0.5 {user.planLabel === 'Pro'
					? 'select-none opacity-50'
					: ''}">{user.planLabel}</Badge
			>
		{/if}
	</div>
	{#if !isMac}
		<div class="window-controls">
			<Button
				variant="ghost"
				size="icon"
				class="window-control-btn"
				onclick={minimize}
				aria-label="Minimize"
			>
				<MinusIcon class="size-3.5" />
			</Button>
			<Button
				variant="ghost"
				size="icon"
				class="window-control-btn"
				onclick={toggleMaximize}
				aria-label={maximized ? 'Restore' : 'Maximize'}
			>
				{#if maximized}
					<CopyIcon class="size-3" />
				{:else}
					<SquareIcon class="size-3" />
				{/if}
			</Button>
			<Button
				variant="ghost"
				size="icon"
				class="window-control-btn window-control-close"
				onclick={close}
				aria-label="Close"
			>
				<XIcon class="size-3.5" />
			</Button>
		</div>
	{/if}
</div>

<style lang="postcss">
	.titlebar {
		display: flex;
		flex-shrink: 0;
		align-items: stretch;
		height: var(--titlebar-height);
		min-height: var(--titlebar-height);
		user-select: none;
		-webkit-user-select: none;
	}

	/* Reserve room for the native macOS traffic lights at the top-left. */
	.titlebar-mac {
		padding-left: 76px;
	}

	.titlebar-fill {
		flex: 1 1 0;
		min-width: 0;
	}

	.titlebar-content {
		display: flex;
		align-items: center;
		min-width: 0;
		height: 100%;
		padding: 0 0.5rem;
		gap: 0.5rem;
	}

	.window-controls {
		display: flex;
		flex-shrink: 0;
		align-items: stretch;
		height: 100%;
	}

	/*
	 * Fluent (Windows 11) standard caption-button hit target: 46x32. We use the
	 * same width on Linux so the bar feels consistent across platforms; the
	 * height tracks the titlebar via `h-full`.
	 */
	.window-controls :global(.window-control-btn) {
		width: 46px;
		height: 100%;
		border-radius: 0;
	}

	.window-controls :global(.window-control-close:hover) {
		background-color: rgb(232 17 35);
		color: white;
	}
</style>
