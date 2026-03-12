<script lang="ts">
	import MenuBar from '$lib/components/MenuBar.svelte';
	import UserButton from '$lib/components/UserButton.svelte';
	import Footer from '$lib/components/marketing/Footer.svelte';
	import { isAuthenticated } from '$lib/stores/auth.js';
	import { Button } from '@eurora/ui/components/button/index';
	import SiGithub from '@icons-pack/svelte-simple-icons/icons/SiGithub';
	import LogInIcon from '@lucide/svelte/icons/log-in';

	const { children } = $props();
</script>

<MenuBar>
	{#snippet actions()}
		<Button variant="default" href="/download">Download</Button>
		<Button variant="ghost" href="/docs">Docs</Button>
		<Button variant="ghost" href="/pricing">Pricing</Button>
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
	{/snippet}
</MenuBar>

{@render children?.()}

<Footer />
