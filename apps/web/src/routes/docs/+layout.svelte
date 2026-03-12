<script lang="ts">
	import { page } from '$app/state';
	import MenuBar from '$lib/components/MenuBar.svelte';
	import DocsSidebar from '$lib/components/docs/Sidebar.svelte';
	import { docsNavItems } from '$lib/components/docs/nav.js';

	let { children } = $props();
</script>

<div class="flex min-h-screen flex-col">
	<MenuBar>
		{#snippet mobileNav(close)}
			<nav class="flex flex-col gap-1 border-t border-border px-4 pt-3">
				<span
					class="px-3 py-1 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
					>Docs</span
				>
				{#each docsNavItems as item}
					<a
						href={item.url}
						aria-current={item.url === page.url.pathname ? 'page' : undefined}
						class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors
							{item.url === page.url.pathname
							? 'bg-muted text-foreground'
							: 'text-muted-foreground hover:text-primary'}"
						onclick={close}
					>
						{item.title}
					</a>
				{/each}
			</nav>
		{/snippet}
	</MenuBar>

	<div class="flex flex-1 flex-col">
		<div class="mx-auto flex w-full max-w-7xl items-start gap-12 py-10">
			<DocsSidebar />

			<main class="flex-1">
				{@render children?.()}
			</main>
		</div>
	</div>
</div>
