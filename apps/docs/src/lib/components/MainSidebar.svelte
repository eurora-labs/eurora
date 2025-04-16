<script lang="ts">
	import { Calendar, House, Inbox, Search, Settings } from '@lucide/svelte';
	import { Sidebar } from '@eurora/ui';

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
