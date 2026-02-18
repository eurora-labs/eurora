<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import Menubar from '$lib/components/Menubar.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
	import { platform } from '@tauri-apps/plugin-os';
	import { onMount } from 'svelte';

	let taurpcService = inject(TAURPC_SERVICE);
	let roleChecked = $state(false);

	let { children } = $props();
	onMount(() => {
		if (document) {
			document.body.classList.add(`${platform()}-app`);
		}

		taurpcService.auth
			.get_role()
			.then((role) => {
				if (role === 'Free') {
					goto('/onboarding/no-access');
				} else {
					roleChecked = true;
				}
			})
			.catch((error) => {
				console.error('Failed to check user role:', error);
				roleChecked = true;
			});
	});
</script>

{#if roleChecked}
	<Menubar />
	<Sidebar.Provider open={true}>
		<MainSidebar />
		<Sidebar.Inset>
			{@render children?.()}
		</Sidebar.Inset>
	</Sidebar.Provider>
{:else}
	<div class="flex items-center justify-center h-screen">
		<Spinner class="size-8" />
	</div>
{/if}
