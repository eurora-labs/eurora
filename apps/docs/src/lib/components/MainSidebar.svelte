<script lang="ts">
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import HouseIcon from '@lucide/svelte/icons/house';
	import InboxIcon from '@lucide/svelte/icons/inbox';
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import SearchIcon from '@lucide/svelte/icons/search';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	const items = [
		{
			title: 'Chat 1',
			url: '#',
			icon: HouseIcon,
		},
		{
			title: 'Chat 2',
			url: '#',
			icon: InboxIcon,
		},
		{
			title: 'Chat 3',
			url: '#',
			icon: CalendarIcon,
		},
		{
			title: 'Chat 4',
			url: '#',
			icon: SearchIcon,
		},
		{
			title: 'Chat 5',
			url: '#',
			icon: SettingsIcon,
		},
	];
</script>

<Sidebar.Root>
	<Sidebar.Header><EuroraLogo />Eurora Labs</Sidebar.Header>
	<Sidebar.Content
		><Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Welcome</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Getting Started</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Why Eurora</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		<Sidebar.Group>
			<Sidebar.GroupLabel>Components</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>
							{#snippet child({ props })}
								<a href="/components/launcher" {...props}>
									<span>Launcher</span>
								</a>
							{/snippet}
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>
							{#snippet child({ props })}
								<a href="/components/context-chip" {...props}>
									<span>Context Chip</span>
								</a>
							{/snippet}
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		<Sidebar.Group>
			<Sidebar.GroupLabel>Application</Sidebar.GroupLabel>
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
		</Sidebar.Group>
	</Sidebar.Content>
	<Sidebar.Footer />
</Sidebar.Root>
