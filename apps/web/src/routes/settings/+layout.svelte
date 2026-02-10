<script lang="ts">
	import SettingsSidebar from '$lib/components/settings/SettingsSidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Sidebar.Provider open={true}>
	<SettingsSidebar />
	<Sidebar.Inset>
		<Sidebar.Trigger class="fixed bottom-0 left-0 m-2" />
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
