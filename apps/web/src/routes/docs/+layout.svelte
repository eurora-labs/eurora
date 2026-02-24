<script lang="ts">
	import DocsSidebar from '$lib/components/docs/Sidebar.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';

	let { children } = $props();
</script>

<div class="flex min-h-screen flex-col">
	<header class="backdrop-blur">
		<div class="flex h-14 w-full items-center px-6">
			<Button
				variant="link"
				class="decoration-transparent gap-2.5 p-0 font-semibold"
				href="/"
			>
				<EuroraLogo style="width: 2rem; height: 2rem;" />
			</Button>
		</div>
	</header>

	<div class="flex flex-1 flex-col pt-16">
		<div class="mx-auto flex w-full max-w-5xl items-start gap-12 px-8 py-10">
			<DocsSidebar />

			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
	</div>
</div>
