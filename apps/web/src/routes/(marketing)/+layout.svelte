<script lang="ts">
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Alert from '@eurora/ui/components/alert/index';
	import * as EurIcons from '@eurora/ui/custom-icons/index';
	import { ShieldAlert, LogInIcon } from '@lucide/svelte';

	import JoinWaitlist from './join_waitlist.svelte';

	const { children } = $props();

	let showAlert = $state(false);

	function dismissAlert() {
		showAlert = false;
	}
</script>

{#if showAlert}
	<div
		class="bg-background fixed top-0 right-0 left-0 z-50 flex items-center justify-between border-b px-6 py-2"
	>
		<Alert.Root class="flex flex-1 items-center">
			<ShieldAlert class="size-4" />
			<Alert.Title>Heads up!</Alert.Title>
			<Alert.Description
				>All existing text on this website is placeholder and doesn't represent anything
				about the product, it's features, the team behind it, or any other factual
				information.</Alert.Description
			>
		</Alert.Root>
		<Button variant="destructive" class="ml-2" onclick={dismissAlert}>Dismiss</Button>
	</div>
{/if}

<div
	class="bg-transparent fixed right-0 left-0 z-40 flex items-center justify-between px-6 py-4 mt-4"
	style="top: {showAlert ? 'var(--alert-height, 40px)' : '0'};"
>
	<div class="flex items-center gap-2">
		<Button variant="link" href="/" class="decoration-transparent">
			<EurIcons.EuroraLogo style="width: 4rem; height: 4rem;" />
			<span class="text-lg">Eurora Labs</span>
		</Button>
	</div>

	<div class="flex items-center gap-4">
		<!-- <Button variant="ghost" href="/download">Download</Button> -->
		<!-- <Button variant="ghost" href="/features">Features</Button> -->
		<!-- <Button variant="ghost" href="/pricing">Pricing</Button> -->
		<Button variant="ghost" href="/about">About Us</Button>
		<!-- <Button variant="ghost" href="/privacy">Privacy</Button> -->
		<!-- <Button variant="ghost" href="/contact">Contact</Button> -->
		<JoinWaitlist />
		<Button variant="ghost" size="icon" href="https://github.com/Eurora-Labs/eurora">
			<SiGithub />
		</Button>
		<!-- <Button variant="default" href="/download">Get Eurora</Button> -->
		<Button variant="ghost" href="/login">
			Login
			<LogInIcon />
		</Button>
	</div>
</div>

{@render children?.()}
