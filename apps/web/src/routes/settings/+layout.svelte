<script lang="ts">
	import SettingsSidebar from '$components/settings/Sidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Sidebar.Provider class="min-h-min" open={true}>
	<SettingsSidebar />
	<Sidebar.Inset class="min-h-min">
		<Sidebar.Trigger class="fixed bottom-0 left-0 m-[0.5rem]" />
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
