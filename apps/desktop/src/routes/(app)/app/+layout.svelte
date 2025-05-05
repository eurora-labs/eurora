<script lang="ts">
	import '@eurora/ui/global.css';
	import { Sidebar } from '@eurora/ui';
	import MainSidebar from '$lib/components/MainSidebar.svelte';
	import { Breadcrumb } from '@eurora/ui';
	import X from '@lucide/svelte/icons/x';
	import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
	import { Button } from '@eurora/ui';
	let { children } = $props();
</script>

<!-- <Sidebar.Provider class="min-h-min" open={false}>
	<MainSidebar />
	<main>
		<Sidebar.Trigger />
		{@render children?.()}
	</main>
</Sidebar.Provider> -->

<Sidebar.Provider class="min-h-min" open={false}>
	<MainSidebar />
	<Sidebar.Inset class="min-h-min">
		<header class="flex h-16 shrink-0 items-center gap-2 border-b px-4">
			<Sidebar.Trigger class="-ml-1" />
			<Breadcrumb.Root>
				<Breadcrumb.List>
					<Breadcrumb.Item class="hidden md:block">
						<Breadcrumb.Link>Eurora</Breadcrumb.Link>
					</Breadcrumb.Item>
					<Breadcrumb.Separator class="hidden md:block" />
					<Breadcrumb.Item>
						<Breadcrumb.Page>Overlay</Breadcrumb.Page>
					</Breadcrumb.Item>
					<Breadcrumb.Separator class="hidden md:block" />
				</Breadcrumb.List>
			</Breadcrumb.Root>
			<div class="ml-auto flex gap-2">
				<Button size="icon" variant="outline"><RotateCcw /></Button>
				<Button size="icon" variant="outline"><X /></Button>
			</div>
		</header>
		<main class="flex-1 overflow-auto p-4">
			<div class="mx-auto h-full max-w-screen-xl">
				{@render children?.()}
			</div>
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
