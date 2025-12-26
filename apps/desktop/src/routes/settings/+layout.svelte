<script lang="ts">
	import SettingsSidebar from '$components/settings/Sidebar.svelte';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();
</script>

<Sidebar.Provider>
	<SettingsSidebar />

	<main class="w-full">
		{@render children?.()}
	</main>
</Sidebar.Provider>
