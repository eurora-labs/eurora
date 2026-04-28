<script lang="ts">
	import { goto } from '$app/navigation';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import SearchDialog from '@eurora/chat/components/SearchDialog.svelte';
	import SidebarThreadsList from '@eurora/chat/components/SidebarThreadsList.svelte';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import SearchIcon from '@lucide/svelte/icons/search';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const chatService = inject(CHAT_SERVICE);
	const user = inject(USER_SERVICE);
	const sidebarState = useSidebar();

	let searchOpen = $state(false);

	const identityLabel = $derived(user.displayName ?? user.email);
	const avatarLetter = $derived(identityLabel ? identityLabel.charAt(0).toUpperCase() : '');

	onMount(() => {
		chatService.loadThreads(20, 0);
	});

	function handleThreadSelect(threadId: string) {
		sidebarState.setOpenMobile(false);
		if (threadId) {
			goto(`/${threadId}`);
		} else {
			goto('/');
		}
	}

	function createChat() {
		sidebarState.setOpenMobile(false);
		chatService.activeThreadId = undefined;
		goto('/');
	}

	function openSearch() {
		searchOpen = true;
	}

	function handleSearchSelect(threadId: string) {
		sidebarState.setOpenMobile(false);
		goto(`/${threadId}`);
	}

	function openSettings() {
		sidebarState.setOpenMobile(false);
		goto('/settings');
	}

	function logout() {
		sidebarState.setOpenMobile(false);
		user.logout().catch((error) =>
			toast.error(
				`Failed to log out: ${error instanceof Error ? error.message : String(error)}`,
			),
		);
	}
</script>

<Sidebar.Root side="left">
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton onclick={createChat}>
					<SquarePenIcon />
					<span>New chat</span>
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton onclick={openSearch}>
					<SearchIcon />
					<span>Search</span>
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<SidebarThreadsList onThreadSelect={handleThreadSelect} />
	</Sidebar.Content>
	<Sidebar.Footer>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuButton {...props} size="lg">
								<div
									class="flex size-8 shrink-0 items-center justify-center rounded-full bg-sidebar-accent text-sidebar-accent-foreground text-sm font-medium"
									aria-hidden="true"
								>
									{avatarLetter}
								</div>
								<span class="flex-1 truncate text-left text-sm"
									>{identityLabel}</span
								>
								<ChevronUpIcon class="size-4 shrink-0" />
							</Sidebar.MenuButton>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content
						side="top"
						align="start"
						class="w-(--bits-dropdown-menu-anchor-width)"
					>
						<DropdownMenu.Item class="cursor-pointer" onclick={openSettings}>
							<SettingsIcon />
							<span>Settings</span>
						</DropdownMenu.Item>
						<DropdownMenu.Separator />
						<DropdownMenu.Item class="cursor-pointer" onclick={logout}>
							<LogOutIcon />
							<span>Log out</span>
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Footer>
</Sidebar.Root>

<SearchDialog bind:open={searchOpen} onSelect={handleSearchSelect} />
