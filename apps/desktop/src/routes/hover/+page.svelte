<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { createTauRPCProxy } from '$lib/bindings/bindings.js';

	const taurpc = createTauRPCProxy();

	function handleClick() {
		taurpc.window.open_launcher_window();
	}
</script>

<Button
	class="container w-full h-full m-auto flex items-center justify-center cursor-pointer "
	onclick={handleClick}
	size="icon"
	variant="link"
>
	<EuroraLogo class="size-8" />
</Button>

<style lang="postcss">
	@reference 'tailwindcss';
	:global(body) {
		@apply bg-transparent;
	}
</style>
