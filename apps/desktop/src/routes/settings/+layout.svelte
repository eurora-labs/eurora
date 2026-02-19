<script lang="ts">
	import SettingsSidebar from '$components/settings/Sidebar.svelte';
	import { Badge } from '@eurora/ui/components/badge/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { getVersion } from '@tauri-apps/api/app';

	let { children } = $props();

	let version = $state(import.meta.env.DEV ? 'DEV' : '');

	if (!import.meta.env.DEV) {
		getVersion().then((v) => (version = `v${v}`));
	}
</script>

<Sidebar.Provider>
	<SettingsSidebar />

	<main class="w-full">
		{@render children?.()}
	</main>
</Sidebar.Provider>

{#if version}
	<Badge variant="outline" class="fixed bottom-4 right-4 select-none opacity-50">
		{version}
	</Badge>
{/if}
