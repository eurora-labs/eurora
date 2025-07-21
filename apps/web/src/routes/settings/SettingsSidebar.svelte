<script lang="ts" module>
	interface MenuItem {
		title: string;
		url: string;
		icon: any;
		isActive?: boolean;
	}
</script>

<script lang="ts">
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronsLeftRightEllipsis from '@lucide/svelte/icons/chevrons-left-right-ellipsis';
	import KeyboardIcon from '@lucide/svelte/icons/keyboard';
	import WebhookIcon from '@lucide/svelte/icons/webhook';
	import MailIcon from '@lucide/svelte/icons/mail';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { page } from '$app/state';
	import { CreditCardIcon } from '@lucide/svelte';

	let generalItems: MenuItem[] = [
		{
			title: 'General',
			url: '/settings/profile',
			icon: BoltIcon,
		},
	];

	let billingItems: MenuItem[] = [
		{
			title: 'Billing & Invoices',
			url: '/settings/billing',
			icon: CreditCardIcon,
		},
	];

	let documentationItems: MenuItem[] = [
		{
			title: 'Documentation',
			url: '/settings/documentation',
			icon: BookOpenIcon,
		},
	];

	let navigationGeneralItems = $derived(
		generalItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);

	let navigationBillingItems = $derived(
		billingItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);

	let navigationDocumentationItems = $derived(
		documentationItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);
</script>

<Sidebar.Root class="border-none">
	<Sidebar.Content class="overflow-x-hidden">
		<div class="flex items-center pt-2">
			<Button
				variant="link"
				class="decoration-transparent rounded-none w-full flex justify-start gap-2 font-bold"
				href="/"
			>
				<EuroraLogo style="width: 2rem; height: 2rem;" />
				Eurora Labs
			</Button>
		</div>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each navigationGeneralItems as item (item.title)}
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

		<Sidebar.Separator />

		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each navigationBillingItems as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={item.isActive}>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<CreditCardIcon />
										<span>Billing & Invoices</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>

		<Sidebar.Separator />

		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each navigationDocumentationItems as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={item.isActive}>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<BookOpenIcon />
										<span>Documentation</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href="mailto:contact@eurora-labs.com" {...props}>
										<MailIcon />
										<span>Contact us</span>
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
