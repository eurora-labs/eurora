<script lang="ts">
	import { page } from '$app/state';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import MailIcon from '@lucide/svelte/icons/mail';

	const navItems = [
		{ title: 'General', url: '/settings', icon: BoltIcon },
		{ title: 'Billing & Invoices', url: '/settings/billing', icon: CreditCardIcon },
		{ title: 'Documentation', url: '/settings/documentation', icon: BookOpenIcon },
	];

	let items = $derived(
		navItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);
</script>

<nav class="flex flex-wrap items-center gap-1">
	{#each items as item (item.title)}
		<a
			href={item.url}
			class="inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium transition-colors
				{item.isActive
				? 'bg-muted text-foreground'
				: 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'}"
		>
			<item.icon size={16} />
			{item.title}
		</a>
	{/each}

	<Dialog.Root>
		<Dialog.Trigger
			class="inline-flex items-center gap-2 rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted/50 hover:text-foreground"
		>
			<MailIcon size={16} />
			Contact Us
		</Dialog.Trigger>
		<Dialog.Content class="sm:max-w-1/2">
			<Dialog.Header class="items-start">
				<Dialog.Title>Contact Us</Dialog.Title>
			</Dialog.Header>
			<p>
				Feel free to reach out to us at
				<a href="mailto:contact@eurora-labs.com" class="inline w-fit underline"
					>contact@eurora-labs.com</a
				>
				for any inquiries or feedback.
			</p>
		</Dialog.Content>
	</Dialog.Root>
</nav>
