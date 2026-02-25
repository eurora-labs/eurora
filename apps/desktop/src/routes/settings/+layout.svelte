<script lang="ts">
	import SettingsSidebar from '$components/settings/Sidebar.svelte';
	import { Badge } from '@eurora/ui/components/badge/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';
	import { getVersion } from '@tauri-apps/api/app';

	let { children } = $props();

	let version = $state(import.meta.env.DEV ? 'DEV' : '');

	if (!import.meta.env.DEV) {
		getVersion().then((v) => (version = `v${v}`));
	}
</script>

<Sidebar.Provider>
	<SettingsSidebar />

	<main class="relative flex-1 overflow-hidden">
		<ScrollArea class="h-full">
			<div class="mx-auto max-w-2xl px-8 py-8">
				{@render children?.()}
			</div>
		</ScrollArea>

		{#if version}
			<Badge variant="outline" class="absolute bottom-4 right-4 select-none opacity-50">
				{version}
			</Badge>
		{/if}
	</main>
</Sidebar.Provider>
