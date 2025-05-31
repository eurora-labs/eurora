<script lang="ts">
	import Calendar from '@lucide/svelte/icons/calendar';
	import House from '@lucide/svelte/icons/house';
	import Inbox from '@lucide/svelte/icons/inbox';
	import Search from '@lucide/svelte/icons/search';
	import Settings from '@lucide/svelte/icons/settings';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	// Menu items.
	const items = [
		{
			title: 'Chat 1',
			url: '#',
			icon: House
		},
		{
			title: 'Chat 2',
			url: '#',
			icon: Inbox
		},
		{
			title: 'Chat 3',
			url: '#',
			icon: Calendar
		},
		{
			title: 'Chat 4',
			url: '#',
			icon: Search
		},
		{
			title: 'Chat 5',
			url: '#',
			icon: Settings
		}
	];
</script>

<Sidebar.Root>
	<Sidebar.Header>Header</Sidebar.Header>
	<Sidebar.Content>
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
	<Sidebar.Footer>Footer</Sidebar.Footer>
</Sidebar.Root>
