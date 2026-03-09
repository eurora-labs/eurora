<script lang="ts" module>
	export interface InfiniteListProps<T> {
		items: T[];
		label?: string;
		loading?: boolean;
		loadingMore?: boolean;
		hasMore?: boolean;
		onLoadMore?: () => void;
		empty?: import('svelte').Snippet;
		skeleton?: import('svelte').Snippet;
		children: import('svelte').Snippet<[T]>;
	}
</script>

<script lang="ts" generics="T">
	import * as Empty from '$lib/components/empty/index.js';
	import * as Sidebar from '$lib/components/sidebar/index.js';
	import { Skeleton } from '$lib/components/skeleton/index.js';
	import { Spinner } from '$lib/components/spinner/index.js';

	let {
		items,
		label,
		loading = false,
		loadingMore = false,
		hasMore = false,
		onLoadMore,
		empty,
		skeleton,
		children,
	}: InfiniteListProps<T> = $props();

	function intersect(node: HTMLElement, callback: () => void) {
		const observer = new IntersectionObserver(
			(entries) => {
				if (entries[0].isIntersecting) {
					observer.disconnect();
					callback();
				}
			},
			{ rootMargin: '100px' },
		);
		observer.observe(node);
		return { destroy: () => observer.disconnect() };
	}
</script>

{#if loading}
	<Sidebar.Group>
		{#if label}<Sidebar.GroupLabel>{label}</Sidebar.GroupLabel>{/if}
		<Sidebar.GroupContent>
			<div class="flex items-center justify-center py-4">
				<Spinner />
			</div>
		</Sidebar.GroupContent>
	</Sidebar.Group>
{:else if items.length === 0}
	{#if empty}
		{@render empty()}
	{:else}
		<Empty.Root>
			<Empty.Header>
				<Empty.Title>No items</Empty.Title>
			</Empty.Header>
		</Empty.Root>
	{/if}
{:else}
	<Sidebar.Group>
		{#if label}<Sidebar.GroupLabel>{label}</Sidebar.GroupLabel>{/if}
		<Sidebar.GroupContent>
			<Sidebar.Menu>
				{#each items as item}
					{@render children(item)}
				{/each}
				{#if hasMore}
					{#if loadingMore}
						{#if skeleton}
							{@render skeleton()}
						{:else}
							{#each Array(3) as _}
								<Sidebar.MenuItem>
									<div class="px-2 py-1.5">
										<Skeleton class="h-4 w-full" />
									</div>
								</Sidebar.MenuItem>
							{/each}
						{/if}
					{:else if onLoadMore}
						{#key items.length}
							<div use:intersect={onLoadMore}></div>
						{/key}
					{/if}
				{/if}
			</Sidebar.Menu>
		</Sidebar.GroupContent>
	</Sidebar.Group>
{/if}
