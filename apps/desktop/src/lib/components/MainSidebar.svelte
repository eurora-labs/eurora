<script lang="ts">
	import { goto } from '$app/navigation';
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import SearchDialog from '@eurora/chat/components/SearchDialog.svelte';
	import SidebarThreadsList from '@eurora/chat/components/SidebarThreadsList.svelte';
	import { CHAT_SERVICE, ThreadMessages } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LogoutIcon from '@lucide/svelte/icons/log-out';
	import PowerIcon from '@lucide/svelte/icons/power';
	import SearchIcon from '@lucide/svelte/icons/search';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const chatService = inject(CHAT_SERVICE);
	const activityService = inject(ACTIVITY_SERVICE);
	const user = inject(USER_SERVICE);
	const sidebarState = Sidebar.useSidebar();

	let quitDialogOpen = $state(false);
	let searchOpen = $state(false);

	function handleKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			searchOpen = true;
		}
	}

	function getFirstLetterAndCapitalize(name: string) {
		if (!name) return '';
		return name.charAt(0).toUpperCase();
	}

	let threadInitialized = false;
	$effect(() => {
		if (user.authenticated && !threadInitialized) {
			threadInitialized = true;
			chatService.loadThreads(20, 0);
		} else if (!user.authenticated && threadInitialized) {
			threadInitialized = false;
			chatService.destroy();
			chatService.loadingThreads = false;
		}
	});

	// Lazy-fetch the per-activity thread bucket whenever the user's
	// scrolled-to selection changes. The chat service short-circuits
	// repeat calls for cached buckets, so cycling through the rail and
	// back is free after the first visit. We skip index 0 — the
	// most-recent slot is the "no selection" default and never filters.
	$effect(() => {
		if (activityService.activeIndex === 0) return;
		const app = activityService.activeApp;
		if (!app) return;
		if (!user.authenticated) return;
		chatService.loadThreadsForActivity(app.id);
	});

	onMount(() => {
		if (!user.authenticated) {
			chatService.loadingThreads = false;
		}

		return () => {
			chatService.destroy();
			threadInitialized = false;
		};
	});

	// Index 0 is the most-recent slot — treated as "no user selection"
	// so the sidebar falls back to the full chronological list. Only an
	// explicit scroll to a non-top rail position counts as filtering.
	const selectedApp = $derived(
		activityService.activeIndex === 0 ? undefined : activityService.activeApp,
	);
	const activityThreads = $derived(
		selectedApp ? chatService.threadsByActivity.get(selectedApp.id) : undefined,
	);

	// `matched` feeds the pinned group at the top of the sidebar. The
	// pinned group renders even when empty (showing a "No chats"
	// placeholder under the group label) so the per-app filter context
	// stays visible regardless of whether the activity has linked chats.
	const matched = $derived(activityThreads ?? []);
	const matchedIds = $derived(new Set(matched.map((t) => t.thread.id)));

	// The main (paginated) list always shows the chronological flow.
	// When a per-app group is present we filter out its threads so the
	// same chat doesn't appear in both sections.
	const chronologicalThreads = $derived.by<ThreadMessages[]>(() => {
		if (!selectedApp || matched.length === 0) return chatService.threads;
		return chatService.threads.filter((t) => !matchedIds.has(t.thread.id));
	});

	async function createChat() {
		chatService.activeThreadId = undefined;
		goto('/');
	}

	async function quit() {
		quitDialogOpen = false;
		commands.systemQuit().catch((error) => {
			console.error('Failed to quit application:', error);
			toast.error(`The app encountered the following error: ${error}`, {
				description: 'Please quit manually from the tray menu.',
				duration: 5000,
				cancel: {
					label: 'Ok',
					onClick: () => {},
				},
			});
		});
	}

	function handleThreadSelect(threadId: string) {
		chatService.activeThreadId = threadId || undefined;
		if (threadId) {
			goto(`/${threadId}`);
		} else {
			goto('/');
		}
	}

	async function handleThreadDelete(threadId: string): Promise<void> {
		await chatService.deleteThread(threadId);
		if (chatService.activeThreadId === undefined) {
			handleThreadSelect('');
		}
	}
</script>

<Sidebar.Root collapsible="icon" class="border-none">
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton onclick={() => createChat()}>
					<SquarePenIcon />
					<span>New chat</span>
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton onclick={() => (searchOpen = true)}>
					<SearchIcon />
					<span>Search</span>
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		{#if sidebarState.open}
			<SidebarThreadsList
				threads={chronologicalThreads}
				loading={chatService.loadingThreads}
				loadingMore={chatService.loadingMoreThreads}
				hasMore={chatService.hasMoreThreads}
				onLoadMore={() => chatService.loadMoreThreads()}
				activeThreadId={chatService.activeThreadId}
				onThreadSelect={handleThreadSelect}
				onThreadDelete={handleThreadDelete}
				label={selectedApp ? 'Other chats' : 'Chats'}
				pinnedLabel={selectedApp ? `Chats in ${selectedApp.displayName}` : undefined}
				pinnedThreads={selectedApp ? matched : undefined}
				pinnedAccentColor={selectedApp?.accent?.hex}
			/>
		{/if}
	</Sidebar.Content>

	<Sidebar.Footer>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						class="flex items-center gap-2 min-w-0 h-auto px-1 py-1 w-full justify-start cursor-pointer"
					>
						<div
							class="flex size-7 shrink-0 items-center justify-center rounded-full bg-sidebar-accent text-sidebar-accent-foreground text-xs font-medium"
						>
							{getFirstLetterAndCapitalize(user.displayName ?? user.email)}
						</div>
						{#if sidebarState.open}
							<span class="truncate text-sm flex-1 text-left"
								>{user.displayName ?? user.email}</span
							>
							<ChevronUpIcon class="size-4 shrink-0" />
						{/if}
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content side="top" align="start" class="w-56">
				<DropdownMenu.Item class="cursor-pointer" onclick={() => goto('/settings')}>
					<SettingsIcon />
					<span>Settings</span>
				</DropdownMenu.Item>
				<DropdownMenu.Sub>
					<DropdownMenu.SubTrigger class="cursor-pointer">
						<PowerIcon />
						<span>Power</span>
					</DropdownMenu.SubTrigger>
					<DropdownMenu.SubContent class="w-40">
						<DropdownMenu.Item
							class="cursor-pointer"
							onclick={() => {
								user.logout().then(() => {
									goto('/onboarding/login');
								});
							}}
						>
							<LogoutIcon />
							<span>Log Out</span>
						</DropdownMenu.Item>
						<DropdownMenu.Item
							class="cursor-pointer"
							onclick={() => (quitDialogOpen = true)}
						>
							<PowerIcon />
							<span>Quit</span>
						</DropdownMenu.Item>
					</DropdownMenu.SubContent>
				</DropdownMenu.Sub>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.Footer>
</Sidebar.Root>

<svelte:window onkeydown={handleKeydown} />

<SearchDialog bind:open={searchOpen} onSelect={(threadId) => goto(`/${threadId}`)} />

<Dialog.Root bind:open={quitDialogOpen}>
	<Dialog.Content class="sm:max-w-100">
		<div class="flex gap-4">
			<div class="shrink-0">
				<EuroraLogo class="size-12" />
			</div>
			<div class="flex flex-col text-left">
				<Dialog.Header class="text-left">
					<Dialog.Title class="text-left">Quit Application</Dialog.Title>
					<Dialog.Description class="text-left">
						Are you sure you want to quit? Any unsaved changes will be lost.
					</Dialog.Description>
				</Dialog.Header>
			</div>
		</div>
		<Dialog.Footer class="gap-2 sm:gap-0">
			<Dialog.Close class={buttonVariants({ variant: 'outline' })}>Cancel</Dialog.Close>
			<Button variant="destructive" onclick={quit}>Quit</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
