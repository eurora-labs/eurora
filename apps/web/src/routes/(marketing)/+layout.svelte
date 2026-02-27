<script lang="ts">
	import UserButton from '$lib/components/UserButton.svelte';
	import { isAuthenticated } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import SiDiscord from '@icons-pack/svelte-simple-icons/icons/SiDiscord';
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import SiX from '@icons-pack/svelte-simple-icons/icons/SiX';
	import LinkedIn from '@lucide/svelte/icons/linkedin';
	import LogInIcon from '@lucide/svelte/icons/log-in';

	const { children } = $props();

	const footerLinks = {
		product: {
			title: 'Product',
			links: [
				{ name: 'Download', href: '/download' },
				{ name: 'Pricing', href: '/pricing' },
				{ name: 'Browser Extension', href: '/download/browser-extension' },
				{ name: 'Roadmap', href: '/roadmap' },
			],
		},
		company: {
			title: 'Company',
			links: [
				// { name: 'About', href: '/about' },
				// { name: 'Careers', href: '/careers' },
				{ name: 'Privacy Policy', href: '/privacy-policy' },
				// { name: 'Terms of Service', href: '/terms' },
			],
		},
		resources: {
			title: 'Resources & Support',
			links: [
				{ name: 'Documentation', href: '/docs' },
				// { name: 'Help Center', href: '/help' },
				{ name: 'Contact', href: '/contact' },
				// { name: 'Blog', href: '/blog' },
			],
		},
		social: {
			title: 'Social',
			links: [
				{ name: 'GitHub', href: 'https://github.com/eurora-labs/eurora', external: true },
				{ name: 'X (Twitter)', href: 'https://x.com/euroralabs', external: true },
				{ name: 'Discord', href: 'https://discord.gg/xRT9EpBEwc', external: true },
				{
					name: 'LinkedIn',
					href: 'https://linkedin.com/company/eurora-labs',
					external: true,
				},
			],
		},
	};
</script>

<div class="bg-transparent z-0 flex items-center justify-between px-6 py-4 mt-2">
	<div class="flex items-center gap-2">
		<Button variant="link" href="/" class="decoration-transparent">
			<EuroraLogo style="width: 2rem; height: 2rem;" />
			<span class="text-lg text-primary-foreground font-bold">Eurora</span>
		</Button>
	</div>

	<div class="flex items-center gap-4">
		<!-- <Button variant="ghost" href="/about">About Us</Button> -->
		<Button variant="ghost" href="/pricing">Pricing</Button>
		<Button variant="ghost" href="/roadmap">Roadmap</Button>

		<Button variant="default" href="/download">Download</Button>
		<Button variant="ghost" size="icon" href="https://github.com/eurora-labs/eurora">
			<SiGithub />
		</Button>
		{#if $isAuthenticated}
			<UserButton />
		{:else}
			<Button variant="outline" href="/login" class="backdrop-blur-2xl">
				Login
				<LogInIcon />
			</Button>
		{/if}
	</div>
</div>

{@render children?.()}

<footer class="border-t border-border bg-background mt-auto">
	<div class="mx-auto max-w-7xl px-6 py-12 lg:px-8">
		<div class="grid grid-cols-2 gap-8 md:grid-cols-4">
			<div>
				<h3 class="text-sm font-semibold text-foreground">{footerLinks.product.title}</h3>
				<ul class="mt-4 space-y-3">
					{#each footerLinks.product.links as link}
						<li>
							<a
								href={link.href}
								class="text-sm text-muted-foreground hover:text-foreground transition-colors"
							>
								{link.name}
							</a>
						</li>
					{/each}
				</ul>
			</div>

			<div>
				<h3 class="text-sm font-semibold text-foreground">{footerLinks.company.title}</h3>
				<ul class="mt-4 space-y-3">
					{#each footerLinks.company.links as link}
						<li>
							<a
								href={link.href}
								class="text-sm text-muted-foreground hover:text-foreground transition-colors"
							>
								{link.name}
							</a>
						</li>
					{/each}
				</ul>
			</div>

			<div>
				<h3 class="text-sm font-semibold text-foreground">{footerLinks.resources.title}</h3>
				<ul class="mt-4 space-y-3">
					{#each footerLinks.resources.links as link}
						<li>
							<a
								href={link.href}
								class="text-sm text-muted-foreground hover:text-foreground transition-colors"
							>
								{link.name}
							</a>
						</li>
					{/each}
				</ul>
			</div>

			<div>
				<h3 class="text-sm font-semibold text-foreground">{footerLinks.social.title}</h3>
				<ul class="mt-4 space-y-3">
					{#each footerLinks.social.links as link}
						<li>
							<a
								href={link.href}
								target="_blank"
								rel="noopener noreferrer"
								class="text-sm text-muted-foreground hover:text-foreground transition-colors inline-flex items-center gap-2"
							>
								{#if link.name === 'GitHub'}
									<SiGithub size={24} />
								{:else if link.name === 'X (Twitter)'}
									<SiX size={24} />
								{:else if link.name === 'Discord'}
									<SiDiscord size={24} />
								{:else if link.name === 'LinkedIn'}
									<LinkedIn class="h-6 w-6" />
								{/if}
								{link.name}
							</a>
						</li>
					{/each}
				</ul>
			</div>
		</div>

		<div
			class="mt-12 border-t border-border pt-8 flex flex-col md:flex-row items-center justify-between gap-4"
		>
			<div class="flex items-center gap-2">
				<EuroraLogo style="width: 1.5rem; height: 1.5rem;" />
				<span class="text-sm font-semibold text-foreground">Eurora Labs</span>
			</div>
			<p class="text-sm text-muted-foreground">
				&copy; {new Date().getFullYear()} Eurora Labs. All rights reserved.
			</p>
		</div>
	</div>
</footer>
