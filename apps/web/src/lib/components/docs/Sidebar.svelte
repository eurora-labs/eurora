<script lang="ts">
	import { page } from '$app/state';

	const navItems = [
		{ title: 'Welcome', url: '/docs' },
		{ title: 'Why Eurora', url: '/docs/why-eurora' },
		{ title: 'Self-Hosting', url: '/docs/self-hosting' },
	];

	let items = $derived(
		navItems.map((item) => ({ ...item, isActive: item.url === page.url.pathname })),
	);
</script>

<nav class="flex w-56 shrink-0 flex-col gap-0.5">
	{#each items as item (item.title)}
		<a
			href={item.url}
			aria-current={item.isActive ? 'page' : undefined}
			class="rounded-lg px-3 py-2 text-sm font-medium transition-colors
				{item.isActive ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'}"
		>
			{item.title}
		</a>
	{/each}
</nav>
