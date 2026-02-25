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
	import { SiBluesky, SiDiscord, SiGithub, SiReddit, SiX } from '@icons-pack/svelte-simple-icons';
	import BoltIcon from '@lucide/svelte/icons/bolt';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import MailIcon from '@lucide/svelte/icons/mail';
	import ServerIcon from '@lucide/svelte/icons/server';
	import { toast } from 'svelte-sonner';

	let contactDialogOpen = $state(false);
	const email = 'contact@eurora-labs.com';

	async function copyEmail() {
		try {
			await navigator.clipboard.writeText(email);
			toast.success('Email copied to clipboard');
		} catch {
			toast.error('Failed to copy email');
		}
	}

	const socials = [
		{ name: 'Website', href: 'https://eurora-labs.com', icon: GlobeIcon },
		{ name: 'GitHub', href: 'https://github.com/eurora-labs/eurora', icon: SiGithub },
		{ name: 'Discord', href: 'https://discord.gg/xRT9EpBEwc', icon: SiDiscord },
		{ name: 'Reddit', href: 'https://reddit.com/r/eurora', icon: SiReddit },
		{
			name: 'Bluesky',
			href: 'https://bsky.app/profile/euroralabs.bsky.social',
			icon: SiBluesky,
		},
		{ name: 'X', href: 'https://x.com/euroralabs', icon: SiX },
	];

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

<Dialog.Root bind:open={contactDialogOpen}>
	<Dialog.Content class="sm:max-w-sm">
		<Dialog.Header>
			<Dialog.Title>Contact Us</Dialog.Title>
			<Dialog.Description>
				<Button
					class="inline-flex items-center gap-1.5 font-mono text-sm hover:text-foreground transition-colors cursor-pointer"
					variant="ghost"
					onclick={copyEmail}
				>
					{email}
					<CopyIcon class="size-3.5" />
				</Button>
			</Dialog.Description>
		</Dialog.Header>
		<div class="flex flex-wrap gap-2">
			{#each socials as social}
				<Button
					variant="outline"
					href={social.href}
					target="_blank"
					rel="noopener noreferrer"
					aria-label={social.name}
					class="size-12"
				>
					<social.icon class="size-5" />
				</Button>
			{/each}
		</div>
	</Dialog.Content>
</Dialog.Root>
