<script lang="ts">
	import { Euro } from 'lucide-svelte';
	import { SiGithub } from '@icons-pack/svelte-simple-icons';
	import { Button, EurIcons, Alert } from '@eurora/ui';
	import { Terminal, X, ShieldAlert } from 'lucide-svelte';

	const { children } = $props();

	let showAlert = $state(true);

	function dismissAlert() {
		showAlert = false;
	}
</script>

{#if showAlert}
	<div
		class="fixed top-0 left-0 right-0 z-50 flex items-center justify-between px-6 py-2 bg-background border-b"
	>
		<Alert.Root class="flex-1 flex items-center">
			<ShieldAlert class="size-4" />
			<Alert.Title>Heads up!</Alert.Title> 
			<Alert.Description>All existing text on this website is placeholder and doesn't represent anything about the product, it's features, the team behind it, or any other factual information.</Alert.Description>
		</Alert.Root>
		<Button variant="destructive" class="ml-2" onclick={dismissAlert}>
			Dismiss
		</Button>
	</div>
{/if}

<div
	class="fixed left-0 right-0 z-40 flex items-center justify-between px-6 py-4 bg-background"
	style="top: {showAlert ? 'var(--alert-height, 40px)' : '0'};"
>
	<div class="flex items-center gap-2">
		<Button variant="link" href="/" class="no-underline">
			<EurIcons.EuroraLogo />
			<span class="text-lg font-semibold no-underline">Eurora</span>
		</Button>
	</div>

	<div class="flex items-center gap-4">
		<Button variant="ghost" href="/eurora/download">Download</Button>
		<Button variant="ghost" href="/eurora/features">Features</Button>
		<Button variant="ghost" href="/eurora/pricing">Pricing</Button>
		<Button variant="ghost" href="/eurora/about">About Us</Button>
		<Button variant="ghost" href="/eurora/privacy">Privacy</Button>
		<Button variant="ghost" href="/eurora/contact">Contact</Button>
		<Button variant="ghost" size="icon">
			<SiGithub />
		</Button>
		<Button variant="default" href="/eurora/download">Get Eurora</Button>
	</div>
</div>

<div style="margin-top: {showAlert ? '6rem' : '4rem'};">
	{@render children?.()}
</div>

<style>
	/* Ensure the content below the header doesn't get hidden */
	:global(body) {
		padding-top: 1rem;
	}
</style>
