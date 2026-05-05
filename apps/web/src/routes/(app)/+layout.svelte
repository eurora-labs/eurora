<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import { AUTH_SERVICE } from '$lib/services/auth-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	let { children } = $props();

	const auth = inject(AUTH_SERVICE);

	$effect(() => {
		if (!auth.isAuthenticated) {
			const redirect = encodeURIComponent(page.url.pathname + page.url.search);
			goto(`/login?redirect=${redirect}`);
		}
	});
</script>

<Sidebar.Provider>
	<MainSidebar />
	<Sidebar.Inset>
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
