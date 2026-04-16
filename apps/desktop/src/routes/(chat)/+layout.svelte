<script lang="ts">
	import { goto } from '$app/navigation';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';

	const user = inject(USER_SERVICE);

	let { children } = $props();

	$effect(() => {
		if (user.authenticated && !user.emailVerified) {
			goto('/onboarding/login/verify-email?redirect=/');
		}
	});
</script>

<Menubar />
<Sidebar.Provider open={true}>
	<MainSidebar />
	<Sidebar.Inset>
		{@render children?.()}
	</Sidebar.Inset>
</Sidebar.Provider>
