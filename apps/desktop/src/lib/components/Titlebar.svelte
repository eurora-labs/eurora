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

	let maximized = $state(false);
	let isMac = platform() === 'macos';

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

<div data-tauri-drag-region class="titlebar bg-background" class:titlebar-mac={isMac}>
	<div class="flex-1" data-tauri-drag-region></div>
	<div class="pointer-events-auto flex items-center gap-2 h-full px-2">
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
		{#if user.authenticated}
			<Badge
				variant={user.planLabel === 'Pro' ? 'outline' : 'secondary'}
				class="text-xs px-2 py-0.5 {user.planLabel === 'Pro'
					? 'select-none opacity-50'
					: ''}">{user.planLabel}</Badge
			>
		{/if}
	</div>
	{#if !isMac}
		<div class="flex items-center h-full">
			<Button
				variant="ghost"
				size="icon"
				class="h-full rounded-none"
				onclick={minimize}
				aria-label="Minimize"
			>
				<MinusIcon class="size-3.5" />
			</Button>
			<Button
				variant="ghost"
				size="icon"
				class="h-full rounded-none"
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
				class="h-full rounded-none"
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
		align-items: center;
		justify-content: flex-end;
		height: var(--titlebar-height);
		min-height: var(--titlebar-height);
		user-select: none;
		-webkit-user-select: none;
	}

	.titlebar-mac {
		pointer-events: none;
	}
</style>
