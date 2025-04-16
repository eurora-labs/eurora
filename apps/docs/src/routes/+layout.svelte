<script lang="ts">
	import '../app.pcss';
	import { MainSidebar } from '@eurora/main-sidebar';
	import { Sidebar, Breadcrumb } from '@eurora/ui';

	let { children } = $props();
</script>

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
		</header>
		<main class="flex-1 overflow-auto p-4">
			<div class="mx-auto h-full max-w-screen-xl">
				{@render children?.()}
			</div>
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
<!-- 
<main class="flex-1 overflow-hidden bg-transparent">
	{@render children?.()}
</main> -->
