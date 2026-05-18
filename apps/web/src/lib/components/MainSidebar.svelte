<script lang="ts">
	import { goto } from '$app/navigation';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import SearchDialog from '@eurora/chat/components/SearchDialog.svelte';
	import SidebarThreadsList from '@eurora/chat/components/SidebarThreadsList.svelte';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LogoutIcon from '@lucide/svelte/icons/log-out';
	import PanelLeftIcon from '@lucide/svelte/icons/panel-left';
	import SearchIcon from '@lucide/svelte/icons/search';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';

	const auth = inject(AUTH_SERVICE);
	const chatService = inject(CHAT_SERVICE);
	const sidebarState = useSidebar();

	let logoHovered = $state(false);
	let searchOpen = $state(false);

	const accountLabel = $derived(auth.user?.display_name ?? auth.user?.email ?? '');
	const accountInitial = $derived(accountLabel ? accountLabel.charAt(0).toUpperCase() : '');

	function handleKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			searchOpen = true;
		}
	}

	let threadsInitialized = false;
	$effect(() => {
		if (auth.isAuthenticated && !threadsInitialized) {
			threadsInitialized = true;
			chatService.loadThreads(20, 0);
		} else if (!auth.isAuthenticated && threadsInitialized) {
			threadsInitialized = false;
			chatService.destroy();
			chatService.loadingThreads = false;
		}
	});

	onMount(() => {
		if (!auth.isAuthenticated) {
			chatService.loadingThreads = false;
		}

		return () => {
			chatService.destroy();
			threadsInitialized = false;
		};
	});

	function createChat() {
		chatService.activeThreadId = undefined;
		goto('/chat');
	}

	function handleThreadSelect(threadId: string) {
		if (threadId) {
			goto(`/chat/${threadId}`);
		} else {
			goto('/chat');
		}
	}

	function logOut() {
		auth.logout();
		goto('/login');
	}
</script>

<Sidebar.Root collapsible="icon" class="border-none">
	<Sidebar.Header>
		<div class="flex items-center justify-between">
			{#if sidebarState.open}
				<EuroraLogo class="size-7" />
			{:else}
				<Button
					variant="ghost"
					size="icon"
					class="size-7"
					onmouseenter={() => (logoHovered = true)}
					onmouseleave={() => (logoHovered = false)}
					onclick={() => {
						sidebarState.toggle();
						logoHovered = false;
					}}
				>
					<PanelLeftIcon class={logoHovered ? 'size-4' : 'hidden'} />
					<EuroraLogo class={logoHovered ? 'hidden' : 'size-7'} />
				</Button>
			{/if}

			{#if sidebarState.open}
				<Sidebar.Trigger />
			{/if}
		</div>
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton onclick={createChat}>
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
			</Sidebar.GroupContent>
		</Sidebar.Group>
		{#if sidebarState.open}
			<SidebarThreadsList
				threads={chatService.threads}
				loading={chatService.loadingThreads}
				loadingMore={chatService.loadingMoreThreads}
				hasMore={chatService.hasMoreThreads}
				onLoadMore={() => chatService.loadMoreThreads()}
				activeThreadId={chatService.activeThreadId}
				onThreadSelect={handleThreadSelect}
				onThreadDelete={async (id) => {
					await chatService.deleteThread(id);
					if (chatService.activeThreadId === undefined) {
						handleThreadSelect('');
					}
				}}
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
							{accountInitial}
						</div>
						{#if sidebarState.open}
							<span class="truncate text-sm flex-1 text-left">{accountLabel}</span>
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
				<DropdownMenu.Item class="cursor-pointer" onclick={logOut}>
					<LogoutIcon />
					<span>Log out</span>
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.Footer>
</Sidebar.Root>

<svelte:window onkeydown={handleKeydown} />

<SearchDialog bind:open={searchOpen} onSelect={(threadId) => goto(`/chat/${threadId}`)} />
