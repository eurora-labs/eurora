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
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import MailIcon from '@lucide/svelte/icons/mail';

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

<Sidebar.Root>
	<Sidebar.Header>
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
	</Sidebar.Header>
	<Sidebar.Content>
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
								<Dialog.Root>
									<Dialog.Trigger
										class="flex flex-row gap-2 text-sm items-center w-full h-full"
									>
										<MailIcon size={16} />
										Contact Us</Dialog.Trigger
									>
									<Dialog.Content class="sm:max-w-1/2">
										<Dialog.Header class="items-start">
											<Dialog.Title>Contact Us</Dialog.Title>
										</Dialog.Header>
										<p>
											Feel free to reach out to us at
											<a
												href="mailto:contact@eurora-labs.com"
												class="inline w-fit underline"
												>contact@eurora-labs.com</a
											>
											for any inquiries or feedback.
										</p>
									</Dialog.Content>
								</Dialog.Root>
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
</Sidebar.Root>
