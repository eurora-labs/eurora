<script lang="ts">
	import SettingsSidebar from '$lib/components/settings/SettingsSidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Sidebar.Provider open={true}>
	<SettingsSidebar />
	<main>
		{@render children?.()}
	</main>
</Sidebar.Provider>
