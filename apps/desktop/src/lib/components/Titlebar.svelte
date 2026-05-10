<script lang="ts">
	import { page } from '$app/state';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import SquareIcon from '@lucide/svelte/icons/square';
	import XIcon from '@lucide/svelte/icons/x';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	const chatService = inject(CHAT_SERVICE);
	const user = inject(USER_SERVICE);
	const sidebar = Sidebar.useSidebar();

	// Thread title and date are meaningful only on the chat page
	// (`(chat)/[[id]]/+page.svelte`). On onboarding, settings, and the
	// `no-access` subroutes there is no active thread, so suppress the
	// labels entirely rather than rendering placeholder values.
	const isChatRoute = $derived(page.route.id === '/(chat)/[[id]]');
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
	carries the attribute. Interactive controls (Tabs, Sidebar.Trigger,
	Min/Max/Close) omit it so they remain clickable.
-->
<div data-tauri-drag-region class="titlebar bg-background" class:titlebar-mac={isMac}>
	<!--
		Leading region holds the sidebar trigger. Its width tracks the
		sidebar so the trigger sits flush with the sidebar's right edge
		when expanded, and falls back to just past the macOS traffic
		lights (or the window's left edge on Windows/Linux) when
		collapsed. Both transitions match the sidebar's 200ms ease-linear.
	-->
	<div data-tauri-drag-region class="titlebar-leading" data-state={sidebar.state}>
		<Sidebar.Trigger />
	</div>
	<div data-tauri-drag-region class="titlebar-fill"></div>
	<div data-tauri-drag-region class="titlebar-content">
		{#if isChatRoute}
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
		{/if}
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
		z-index: 20;
		/*
		 * The expanded sidebar (z-10, absolute at top:0 of app-shell)
		 * extends up under the Titlebar. Lift the bar above it so the
		 * trigger and window controls remain hittable; the sidebar
		 * background is the same token as bg-background, so visually
		 * the trigger still reads as part of the sidebar.
		 */
		position: relative;
		flex-shrink: 0;
		align-items: stretch;
		height: var(--titlebar-height);
		min-height: var(--titlebar-height);
		user-select: none;
		-webkit-user-select: none;
	}

	/*
	 * macOS traffic lights center slightly above the geometric center
	 * of a 32px bar (their button center sits ~1px above y=16). Borrow
	 * 2px from the bottom via padding so all centered children shift
	 * up by 1px and meet the lights' visual center. The bar's outer
	 * box stays 32px (with `box-sizing: border-box`), so the sidebar's
	 * `padding-top: var(--titlebar-height)` and the app-shell layout
	 * are unaffected.
	 */
	.titlebar-mac {
		padding-bottom: 4px;
	}

	/*
	 * Leading region: hosts the sidebar trigger, right-aligned. The
	 * region's width is driven by the sidebar's open state so the
	 * trigger sits flush with the sidebar's right edge when expanded
	 * and against the window's leading edge when collapsed. The macOS
	 * traffic-light reservation (76px padding-left, with width grown
	 * to compensate under border-box) applies only in the collapsed
	 * state; once expanded, the sidebar background already extends
	 * past the traffic lights.
	 *
	 * Collapsed width is `--leading-collapsed-w` (1.75rem trigger +
	 * 0.25rem right inset = 2rem). Timing matches the sidebar's own
	 * collapse animation (`transition-[width] duration-200 ease-linear`
	 * in sidebar.svelte) so the trigger and sidebar move in lockstep.
	 */
	.titlebar-leading {
		--leading-collapsed-w: 2rem;
		display: flex;
		flex-shrink: 0;
		align-items: center;
		justify-content: flex-end;
		width: var(--leading-collapsed-w);
		padding-right: 0.25rem;
		transition:
			width 200ms linear,
			padding-left 200ms linear;
	}

	.titlebar-leading[data-state='expanded'] {
		width: var(--sidebar-width);
	}

	.titlebar-mac .titlebar-leading[data-state='collapsed'] {
		width: calc(var(--leading-collapsed-w) + 76px);
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
