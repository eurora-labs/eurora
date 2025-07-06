<script lang="ts">
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import SettingsSidebar from './Sidebar.svelte';

	let { children } = $props();
</script>

<Sidebar.Provider>
	<SettingsSidebar />

	<main>
		{@render children?.()}
	</main>
</Sidebar.Provider>
