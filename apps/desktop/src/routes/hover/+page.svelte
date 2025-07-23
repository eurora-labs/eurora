<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { Window } from '@tauri-apps/api/window';
	import { onMount } from 'svelte';

	let window: Window | null = $state(null);

	onMount(() => {
		window = new Window('launcher');
	});

	function handleClick() {
		window?.show();
		window?.setFocus();
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
