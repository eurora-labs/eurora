<script lang="ts">
	import { page } from '$app/state';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { useTauriListen } from '$lib/utils/use-tauri-listen.js';
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

	const chatService = inject(CHAT_SERVICE);
	const user = inject(USER_SERVICE);
	const sidebar = Sidebar.useSidebar();

	// Thread title and date are meaningful only on the chat page
	// (`(chat)/[[id]]/+page.svelte`). On onboarding, settings, and the
	// `no-access` subroutes there is no active thread, so suppress the
	// labels entirely rather than rendering placeholder values.
	const isChatRoute = $derived(page.route.id === '/(chat)/[[id]]');
	// The sidebar trigger is meaningful anywhere `MainSidebar` is mounted,
	// which is the entire `(chat)` route group (chat page plus the
	// `no-access` subroutes). Outside that group there is no sidebar to
	// toggle, so the trigger and the leading region collapse away.
	const showSidebarTrigger = $derived(page.route.id?.startsWith('/(chat)') ?? false);
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

	appWindow.isMaximized().then((val) => (maximized = val));
	useTauriListen(() =>
		appWindow.onResized(async () => {
			maximized = await appWindow.isMaximized();
		}),
	);

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
		sidebar (see .titlebar-leading) so the trigger's center matches
		what's beneath it: flush with the sidebar's right edge when
		expanded, and centered over the sidebar's icon column when
		collapsed. Transitions match the sidebar's 200ms ease-linear.
	-->
	<div
		data-tauri-drag-region
		class="titlebar-leading"
		data-state={sidebar.state}
		data-trigger={showSidebarTrigger ? 'visible' : 'hidden'}
	>
		{#if showSidebarTrigger}
			<Sidebar.Trigger class="size-8" />
		{/if}
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
	 * Leading region: hosts the sidebar trigger. Width tracks the
	 * sidebar's own width tokens (defined on .group/sidebar-wrapper in
	 * sidebar-provider.svelte) so the trigger lands in two different
	 * places depending on state:
	 *
	 *   - Expanded: width = --sidebar-width; the trigger is flush
	 *     against the sidebar's right edge (justify-content: flex-end
	 *     with a 0.25rem inset).
	 *   - Collapsed: width = --sidebar-width-icon (3rem); the trigger
	 *     is centered, which aligns its 1.75rem-wide body with the
	 *     icon column of Sidebar.MenuButton in icon mode (whose 16px
	 *     icons center at x = 24px inside the 3rem column).
	 *
	 * Both states use justify-content: flex-end and differ only in
	 * width + padding-right, so the trigger glides smoothly between
	 * positions instead of snapping (justify-content is not
	 * animatable). Timing matches sidebar.svelte's own collapse
	 * animation (transition-[width] duration-200 ease-linear) so the
	 * trigger and sidebar move in lockstep.
	 *
	 * On macOS we reserve 76px on the leading edge when collapsed for
	 * the traffic lights — the lights occupy what would otherwise be
	 * the icon column, so we shift the 3rem region to their right and
	 * keep the trigger centered inside it. Once expanded, the sidebar
	 * background already extends past the lights so no reservation is
	 * needed.
	 */
	.titlebar-leading {
		--trigger-size: 2rem; /* Sidebar.Trigger renders as `size-8` */
		display: flex;
		flex-shrink: 0;
		align-items: center;
		justify-content: flex-end;
		transition:
			width 200ms linear,
			padding-right 200ms linear,
			padding-left 200ms linear;
	}

	.titlebar-leading[data-state='expanded'] {
		width: var(--sidebar-width);
		padding-right: 0.25rem;
	}

	.titlebar-leading[data-state='collapsed'] {
		width: var(--sidebar-width-icon);
		padding-right: calc((var(--sidebar-width-icon) - var(--trigger-size)) / 2);
	}

	.titlebar-mac .titlebar-leading[data-state='collapsed'] {
		width: calc(var(--sidebar-width-icon) + 76px);
		padding-left: 76px;
	}

	/*
	 * Outside the (chat) route group, `MainSidebar` is not mounted and the
	 * trigger is hidden. Collapse the leading region so titlebar content
	 * starts at the window edge. On macOS we still need to clear the
	 * traffic lights, so reserve the same 76px gutter used for the
	 * collapsed-sidebar case.
	 */
	.titlebar-leading[data-trigger='hidden'] {
		width: 0;
		padding-right: 0;
		padding-left: 0;
	}

	.titlebar-mac .titlebar-leading[data-trigger='hidden'] {
		width: 76px;
		padding-left: 0;
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
