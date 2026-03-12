<script lang="ts">
	import MenuBar from '$lib/components/MenuBar.svelte';
	import DocsSidebar from '$lib/components/docs/Sidebar.svelte';

	let { children } = $props();
</script>

<div class="flex min-h-screen flex-col">
	<MenuBar />

	<div class="flex flex-1 flex-col pt-16">
		<div class="mx-auto flex w-full max-w-7xl items-start gap-12 px-8 py-10">
			<DocsSidebar />

			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
	</div>
</div>
