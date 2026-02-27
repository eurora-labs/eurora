<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { SiBluesky, SiDiscord, SiGithub, SiReddit, SiX } from '@icons-pack/svelte-simple-icons';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import MailIcon from '@lucide/svelte/icons/mail';
	import MapPinIcon from '@lucide/svelte/icons/map-pin';

	const email = 'contact@eurora-labs.com';
	let copied = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText(email);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

	const socials = [
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
</script>

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-24">
	<div class="mb-6">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">Get in touch</p>
		<h1 class="text-4xl font-bold mb-4 sm:text-5xl">We read every message.</h1>
		<p class="max-w-2xl text-lg text-muted-foreground leading-relaxed">
			Whether you have a question, want to report a bug, or just want to say hello — reach out
			through any of the channels below.
		</p>
	</div>

	<Separator class="mb-16" />

	<div class="grid grid-cols-1 gap-12 mb-20 md:grid-cols-2">
		<div>
			<h2 class="text-2xl font-bold mb-6">Email</h2>
			<p class="text-muted-foreground mb-4">
				The fastest way to reach us. We typically respond within one day.
			</p>
			<Button variant="outline" onclick={copyEmail} class="font-mono text-sm">
				{email}
				<CopyIcon class="h-3.5 w-3.5 text-muted-foreground" />
				{#if copied}
					<span class="text-xs text-primary">Copied!</span>
				{/if}
			</Button>
		</div>

		<div>
			<h2 class="text-2xl font-bold mb-6">Office</h2>
			<div class="flex items-start gap-3">
				<MapPinIcon class="h-5 w-5 text-primary mt-0.5 shrink-0" />
				<div class="text-muted-foreground">
					<p class="font-medium text-foreground">Eurora Labs B.V.</p>
					<p>Braillelaan 65</p>
					<p>5252 CW Vlijmen</p>
					<p>The Netherlands</p>
					<p class="text-sm mt-2">Chamber of Commerce: 98516167</p>
				</div>
			</div>
		</div>
	</div>

	<div class="mb-20">
		<h2 class="text-2xl font-bold mb-3">Community</h2>
		<p class="text-muted-foreground max-w-2xl mb-8">
			Join the conversation. Ask questions, share feedback, or connect with other Eurora
			users.
		</p>
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-5">
			{#each socials as social}
				<a
					href={social.href}
					target="_blank"
					rel="noopener noreferrer"
					class="group flex flex-col items-center gap-3 rounded-xl border border-border bg-card p-6 transition-colors hover:border-primary/30"
				>
					<span class="transition-transform group-hover:scale-110"
						><social.icon size={24} /></span
					>
					<span class="text-sm font-medium">{social.name}</span>
				</a>
			{/each}
		</div>
	</div>

	<div class="rounded-2xl bg-foreground/5 p-8 sm:p-12">
		<div class="max-w-lg mx-auto text-center">
			<MailIcon class="h-8 w-8 text-primary mx-auto mb-4" />
			<h2 class="text-2xl font-bold mb-3">Enterprise inquiries</h2>
			<p class="text-muted-foreground mb-6">
				Looking for a custom deployment, dedicated support, or team pricing? We'd love to
				hear about your needs.
			</p>
			<Button href="/pricing" variant="outline">View Pricing</Button>
		</div>
	</div>
</div>
