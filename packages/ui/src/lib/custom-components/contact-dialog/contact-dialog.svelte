<script lang="ts">
	import { Button } from '$lib/components/button/index';
	import * as Dialog from '$lib/components/dialog/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import IconBrandBluesky from '@tabler/icons-svelte-runes/icons/brand-bluesky';
	import IconBrandDiscord from '@tabler/icons-svelte-runes/icons/brand-discord';
	import IconBrandGithub from '@tabler/icons-svelte-runes/icons/brand-github';
	import IconBrandReddit from '@tabler/icons-svelte-runes/icons/brand-reddit';
	import IconBrandX from '@tabler/icons-svelte-runes/icons/brand-x';
	import { toast } from 'svelte-sonner';

	let {
		open = $bindable(false),
		showWebsiteLink = true,
	}: {
		open?: boolean;
		showWebsiteLink?: boolean;
	} = $props();

	const email = 'contact@eurora-labs.com';

	async function copyEmail() {
		try {
			await navigator.clipboard.writeText(email);
			toast.success('Email copied to clipboard');
		} catch {
			toast.error('Failed to copy email');
		}
	}

	const allSocials = [
		{ name: 'Website', href: 'https://eurora-labs.com', icon: GlobeIcon },
		{ name: 'GitHub', href: 'https://github.com/eurora-labs/eurora', icon: IconBrandGithub },
		{ name: 'Discord', href: 'https://discord.gg/xRT9EpBEwc', icon: IconBrandDiscord },
		{ name: 'Reddit', href: 'https://reddit.com/r/eurora', icon: IconBrandReddit },
		{
			name: 'Bluesky',
			href: 'https://bsky.app/profile/euroralabs.bsky.social',
			icon: IconBrandBluesky,
		},
		{ name: 'X', href: 'https://x.com/euroralabs', icon: IconBrandX },
	];

	const socials = $derived(
		showWebsiteLink ? allSocials : allSocials.filter((s) => s.name !== 'Website'),
	);
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="w-fit">
		<Dialog.Header>
			<Dialog.Title>Contact Us</Dialog.Title>
			<Dialog.Description>
				<Button
					class="inline-flex items-center gap-1.5 font-mono text-sm transition-colors hover:text-foreground"
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
