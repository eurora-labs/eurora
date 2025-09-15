<script lang="ts">
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import TextCursorInputIcon from '@lucide/svelte/icons/text-cursor-input';
	import ListIcon from '@lucide/svelte/icons/list';
	const components = [
		{ title: 'Input Box', url: '/components/input-box', icon: TextCursorInputIcon },
		{ title: 'File List', url: '/components/launcher', icon: ListIcon },
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
					{#each components as item (item.title)}
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
