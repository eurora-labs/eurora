<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import SquareIcon from '@lucide/svelte/icons/square';
	import XIcon from '@lucide/svelte/icons/x';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

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

<div data-tauri-drag-region class="titlebar" class:titlebar-mac={isMac}>
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
				aria-label="Maximize"
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

<style>
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
