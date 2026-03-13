<script lang="ts">
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Menubar />
<Sidebar.Provider open={true}>
	<MainSidebar />
	<Sidebar.Inset>
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
