<script lang="ts">
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import LogInIcon from '@lucide/svelte/icons/log-in';
	import UserButton from '$lib/components/UserButton.svelte';
	import { isAuthenticated } from '$lib/stores/auth.js';

	const { children } = $props();
</script>

<div class="bg-transparent z-0 flex items-center justify-between px-6 py-4 mt-2">
	<div class="flex items-center gap-2">
		<Button variant="link" href="/" class="decoration-transparent">
			<EuroraLogo style="width: 2rem; height: 2rem;" />
			<span class="text-lg font-bold">Eurora Labs</span>
		</Button>
	</div>

	<div class="flex items-center gap-4">
		<!-- <Button variant="ghost" href="/features">Features</Button> -->
		<Button variant="ghost" href="/about">About Us</Button>
		<Button variant="ghost" href="/pricing">Pricing</Button>
		<!-- <Button variant="ghost" href="/privacy">Privacy</Button> -->
		<!-- <Button variant="ghost" href="/contact">Contact</Button> -->
		<!-- <JoinWaitlist /> -->

		<Button variant="default" href="/download">Download</Button>
		<Button variant="ghost" size="icon" href="https://github.com/Eurora-Labs/eurora">
			<SiGithub />
		</Button>
		<!-- <Button variant="default" href="/download">Get Eurora</Button> -->
		{#if $isAuthenticated}
			<UserButton />
		{:else}
			<!-- Login -->
			<Button variant="outline" href="/login" class="backdrop-blur-2xl">
				Login
				<LogInIcon />
			</Button>
		{/if}
	</div>
</div>

{@render children?.()}
