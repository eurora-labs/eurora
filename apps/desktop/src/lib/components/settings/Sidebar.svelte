<script lang="ts" module>
	interface MenuItem {
		title: string;
		url: string;
		icon: any;
		isActive?: boolean;
	}
</script>

<script lang="ts">
	import { page } from '$app/state';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { ContactDialog } from '@eurora/ui/custom-components/contact-dialog/index';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import MailIcon from '@lucide/svelte/icons/mail';
	import ServerIcon from '@lucide/svelte/icons/server';

	let contactDialogOpen = $state(false);

	let items: MenuItem[] = [
		{
			title: 'General',
			url: '/settings',
			icon: BoltIcon,
		},
		{
			title: 'API',
			url: '/settings/api',
			icon: ServerIcon,
		},
		// {
		// 	title: 'Telemetry',
		// 	url: '/settings/telemetry',
		// 	icon: ChevronsLeftRightEllipsis,
		// },
	];

	let navigation = $derived(
		items.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
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
	<Sidebar.Footer>
		<Button variant="outline" size="sm" onclick={() => (contactDialogOpen = true)}>
			<MailIcon class="size-4" />
			Contact us
		</Button>
	</Sidebar.Footer>
</Sidebar.Root>

<ContactDialog bind:open={contactDialogOpen} />
