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
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronsLeftRightEllipsis from '@lucide/svelte/icons/chevrons-left-right-ellipsis';
	import BlocksIcon from '@lucide/svelte/icons/blocks';
	import MailIcon from '@lucide/svelte/icons/mail';

	let items: MenuItem[] = [
		{
			title: 'General',
			url: '/settings',
			icon: BoltIcon,
		},
		{
			title: 'Telemetry',
			url: '/settings/telemetry',
			icon: ChevronsLeftRightEllipsis,
		},
		{
			title: 'Third Party',
			url: '/settings/third-party',
			icon: BlocksIcon,
		},
	];

	let navigation = $derived(
		items.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);
</script>

<Sidebar.Root>
	<Sidebar.Content>
		<div class="flex items-center gap-2 pt-2">
			<Button
				variant="ghost"
				class="rounded-none w-full flex justify-start text-[24px] font-normal"
				href="/"
			>
				<ChevronLeftIcon class="size-6" />
				Profile
			</Button>
		</div>
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
		<Button href="mailto:contact@eurora-labs.com">Contact us <MailIcon /></Button>
	</Sidebar.Footer>
</Sidebar.Root>
