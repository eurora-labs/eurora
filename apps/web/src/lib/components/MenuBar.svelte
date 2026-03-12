<script lang="ts">
	import UserButton from '$lib/components/UserButton.svelte';
	import { isAuthenticated } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as NavigationMenu from '@eurora/ui/components/navigation-menu/index';
	import { navigationMenuTriggerStyle } from '@eurora/ui/components/navigation-menu/navigation-menu-trigger.svelte';
	import * as Sheet from '@eurora/ui/components/sheet/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import LogInIcon from '@lucide/svelte/icons/log-in';
	import MenuIcon from '@lucide/svelte/icons/menu';
	import type { Snippet } from 'svelte';

	let { mobileNav }: { mobileNav?: Snippet<[() => void]> } = $props();

	let mobileOpen = $state(false);

	function closeMobile() {
		mobileOpen = false;
	}

	const navLinks = [
		{ href: '/docs/why-eurora', label: 'Why Eurora' },
		{ href: '/docs', label: 'Docs' },
		{ href: '/pricing', label: 'Pricing' },
		{ href: '/about', label: 'About us' },
	];
</script>

<div class="bg-transparent z-0 w-full px-6 py-4 mt-2">
	<div class="mx-auto max-w-7xl flex items-center justify-between">
		<Button variant="link" href="/" class="decoration-transparent">
			<EuroraLogo style="width: 2rem; height: 2rem;" />
			<span class="text-lg text-primary-foreground font-bold">Eurora</span>
		</Button>

		<!-- Desktop nav -->
		<NavigationMenu.Root class="hidden md:flex">
			<NavigationMenu.List>
				{#each navLinks as link}
					<NavigationMenu.Item>
						<NavigationMenu.Link>
							{#snippet child()}
								<a href={link.href} class={navigationMenuTriggerStyle()}
									>{link.label}</a
								>
							{/snippet}
						</NavigationMenu.Link>
					</NavigationMenu.Item>
				{/each}
			</NavigationMenu.List>
		</NavigationMenu.Root>

		<!-- Desktop actions -->
		<div class="hidden md:flex items-center gap-4">
			<Button variant="ghost" size="icon" href="https://github.com/eurora-labs/eurora">
				<SiGithub />
			</Button>
			{#if $isAuthenticated}
				<UserButton />
			{:else}
				<Button variant="ghost" href="/login" class="backdrop-blur-2xl">
					Login
					<LogInIcon />
				</Button>
			{/if}
			<Button variant="default" href="/download">Download</Button>
		</div>

		<!-- Mobile actions -->
		<div class="flex md:hidden items-center gap-3">
			{#if $isAuthenticated}
				<UserButton />
			{:else}
				<Button variant="ghost" href="/login" size="sm" class="backdrop-blur-2xl">
					Login
					<LogInIcon />
				</Button>
			{/if}
			<Sheet.Root bind:open={mobileOpen}>
				<Sheet.Trigger>
					<MenuIcon class="size-6" />
					<span class="sr-only">Open menu</span>
				</Sheet.Trigger>
				<Sheet.Content side="right" class="flex flex-col">
					<Sheet.Header>
						<Sheet.Title>
							<Button
								variant="link"
								href="/"
								class="decoration-transparent"
								onclick={closeMobile}
							>
								<EuroraLogo style="width: 1.5rem; height: 1.5rem;" />
								<span class="text-lg font-bold">Eurora</span>
							</Button>
						</Sheet.Title>
					</Sheet.Header>
					<nav class="flex flex-col gap-1 px-4">
						{#each navLinks as link}
							<a
								href={link.href}
								class="text-foreground hover:text-primary rounded-md px-3 py-2 text-base font-medium transition-colors"
								onclick={closeMobile}
							>
								{link.label}
							</a>
						{/each}
					</nav>
					{#if mobileNav}
						{@render mobileNav(closeMobile)}
					{/if}
					<div class="mt-auto flex flex-col gap-3 px-4 pb-6">
						<Button
							variant="default"
							href="/download"
							onclick={closeMobile}
							class="w-full">Download</Button
						>
						{#if $isAuthenticated}
							<UserButton />
						{:else}
							<Button
								variant="outline"
								href="/login"
								onclick={closeMobile}
								class="w-full"
							>
								Login
								<LogInIcon />
							</Button>
						{/if}
						<Button
							variant="ghost"
							size="icon"
							href="https://github.com/eurora-labs/eurora"
						>
							<SiGithub />
						</Button>
					</div>
				</Sheet.Content>
			</Sheet.Root>
		</div>
	</div>
</div>
