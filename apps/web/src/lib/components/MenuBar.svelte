<script lang="ts">
	import UserButton from '$lib/components/UserButton.svelte';
	import { isAuthenticated } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import * as NavigationMenu from '@eurora/ui/components/navigation-menu/index';
	import { navigationMenuTriggerStyle } from '@eurora/ui/components/navigation-menu/navigation-menu-trigger.svelte';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import LogInIcon from '@lucide/svelte/icons/log-in';
</script>

<div class="bg-transparent z-0 w-full px-6 py-4 mt-2">
	<div class="mx-auto max-w-7xl flex items-center justify-between">
		<Button variant="link" href="/" class="decoration-transparent">
			<EuroraLogo style="width: 2rem; height: 2rem;" />
			<span class="text-lg text-primary-foreground font-bold">Eurora</span>
		</Button>

		<NavigationMenu.Root>
			<NavigationMenu.List>
				<NavigationMenu.Item>
					<NavigationMenu.Link>
						{#snippet child()}
							<a href="/docs/why-eurora" class={navigationMenuTriggerStyle()}
								>Why Eurora</a
							>
						{/snippet}
					</NavigationMenu.Link>
				</NavigationMenu.Item>
				<NavigationMenu.Item>
					<NavigationMenu.Link>
						{#snippet child()}
							<a href="/docs" class={navigationMenuTriggerStyle()}>Docs</a>
						{/snippet}
					</NavigationMenu.Link>
				</NavigationMenu.Item>
				<NavigationMenu.Item>
					<NavigationMenu.Link>
						{#snippet child()}
							<a href="/pricing" class={navigationMenuTriggerStyle()}>Pricing</a>
						{/snippet}
					</NavigationMenu.Link>
				</NavigationMenu.Item>
				<NavigationMenu.Item>
					<NavigationMenu.Link>
						{#snippet child()}
							<a href="/about" class={navigationMenuTriggerStyle()}>About us</a>
						{/snippet}
					</NavigationMenu.Link>
				</NavigationMenu.Item>
			</NavigationMenu.List>
		</NavigationMenu.Root>

		<div class="flex items-center gap-4">
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
			<Button variant="default" href="/download">Download</Button>
		</div>
	</div>
</div>
