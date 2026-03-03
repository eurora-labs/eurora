<script lang="ts">
	import DocsSidebar from '$lib/components/docs/Sidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Sidebar.Provider>
	<DocsSidebar />

	<main class="flex-1">
		<div class="mx-auto max-w-3xl px-8 py-10">
			{@render children?.()}
		</div>
	</main>
</Sidebar.Provider>
