<script lang="ts">
	import { page } from '$app/state';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import ServerIcon from '@lucide/svelte/icons/server';

	const navItems = [
		{ title: 'Overview', url: '/docs', icon: BookOpenIcon },
		{ title: 'Self-Hosting', url: '/docs/self-hosting', icon: ServerIcon },
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
			class="flex items-center gap-2.5 rounded-lg px-3 py-2 text-sm font-medium transition-colors
				{item.isActive ? 'bg-muted text-foreground' : 'text-muted-foreground hover:text-foreground'}"
		>
			<item.icon size={16} />
			{item.title}
		</a>
	{/each}
</nav>
