<script lang="ts">
	import { page } from '$app/state';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';

	const navItems = [
		{ title: 'Welcome', url: '/docs' },
		{ title: 'Why Eurora', url: '/docs/why-eurora' },
		{ title: 'Self-Hosting', url: '/docs/self-hosting' },
	];

	let navigation = $derived(
		navItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);
</script>

<Sidebar.Root class="border-none">
	<Sidebar.Header>
		<Button variant="ghost" size="sm" class="justify-start gap-2" href="/">
			<ChevronLeftIcon class="size-4" />
			<span class="text-sm font-medium">Back</span>
		</Button>
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each navigation as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={item.isActive}>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
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
</Sidebar.Root>
