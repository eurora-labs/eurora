<script lang="ts">
	import Calendar from '@lucide/svelte/icons/calendar';
	import House from '@lucide/svelte/icons/house';
	import Inbox from '@lucide/svelte/icons/inbox';
	import Search from '@lucide/svelte/icons/search';
	import Settings from '@lucide/svelte/icons/settings';
	import CircleUserRoundIcon from '@lucide/svelte/icons/circle-user-round';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { onMount } from 'svelte';

	// type SidebarState = ReturnType<typeof

	let sidebarState: ReturnType<typeof useSidebar> | undefined = undefined;

	onMount(() => {
		sidebarState = useSidebar();
	});

	// Menu items.
	// const items = [
	// 	{
	// 		title: 'Chat 1',
	// 		url: '#',
	// 		icon: House,
	// 	},
	// 	{
	// 		title: 'Chat 2',
	// 		url: '#',
	// 		icon: Inbox,
	// 	},
	// 	{
	// 		title: 'Chat 3',
	// 		url: '#',
	// 		icon: Calendar,
	// 	},
	// 	{
	// 		title: 'Chat 4',
	// 		url: '#',
	// 		icon: Search,
	// 	},
	// 	{
	// 		title: 'Chat 5',
	// 		url: '#',
	// 		icon: Settings,
	// 	},
	// ];
</script>

<Sidebar.Root collapsible="icon" class="border-none">
	<Sidebar.Header>
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				<EuroraLogo class="size-7" onclick={() => sidebarState?.setOpen(true)} />
			</div>

			{#if sidebarState?.open}
				<Sidebar.Trigger />
			{/if}
		</div>
	</Sidebar.Header>
	<Sidebar.Content>
		<!-- <Sidebar.Group>
			<Sidebar.GroupLabel>Chats</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each items as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group> -->
	</Sidebar.Content>
	<Sidebar.Footer>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuButton
								{...props}
								class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
							>
								<CircleUserRoundIcon />
								<span>Profile</span>
								<ChevronUpIcon class="ml-auto" />
							</Sidebar.MenuButton>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content side="top" class="w-(--bits-dropdown-menu-anchor-width)">
						<DropdownMenu.Item>
							{#snippet child({ props })}
								<a {...props} href="/settings">
									<span>Settings</span>
								</a>
							{/snippet}
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Footer>
</Sidebar.Root>
