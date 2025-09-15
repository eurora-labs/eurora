<script lang="ts">
	import '../app.pcss';
	import '@eurora/ui/main.css';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import MainSidebar from '$lib/components/MainSidebar.svelte';

	let { children } = $props();
</script>

<Sidebar.Provider class="min-h-min" open={true}>
	<MainSidebar />
	<Sidebar.Inset class="min-h-min">
		<Sidebar.Trigger class="fixed bottom-0 left-0 m-[0.5rem]" />
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
