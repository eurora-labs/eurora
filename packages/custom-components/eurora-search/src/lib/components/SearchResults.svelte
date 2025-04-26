<script lang="ts">
	import { onMount, afterUpdate } from 'svelte';
	import type { SearchResult } from '../types/search-result';
	import { SearchResultCategory } from '../types/search-result';
	import SearchResultItem from './SearchResultItem.svelte';

	export let results: SearchResult[] = [];
	export let loading = false;
	export let emptyMessage = 'No results found';

	// Keyboard navigation state
	let selectedIndex = -1;
	let resultElements: HTMLElement[] = [];

	// Group results by category
	$: groupedResults = groupByCategory(results);

	// Handle keyboard navigation
	function handleKeyDown(event: KeyboardEvent) {
		if (results.length === 0) return;

		switch (event.key) {
			case 'ArrowDown':
				event.preventDefault();
				selectedIndex = (selectedIndex + 1) % results.length;
				scrollToSelected();
				break;
			case 'ArrowUp':
				event.preventDefault();
				selectedIndex = selectedIndex <= 0 ? results.length - 1 : selectedIndex - 1;
				scrollToSelected();
				break;
			case 'Enter':
				if (selectedIndex >= 0 && selectedIndex < results.length) {
					event.preventDefault();
					const result = results[selectedIndex];
					if (result.action) {
						result.action();
					}
				}
				break;
		}
	}

	// Scroll to the currently selected result
	function scrollToSelected() {
		if (selectedIndex >= 0 && resultElements[selectedIndex]) {
			resultElements[selectedIndex].scrollIntoView({
				behavior: 'smooth',
				block: 'nearest'
			});
		}
	}

	// Group results by category
	function groupByCategory(results: SearchResult[]) {
		const grouped = new Map<SearchResultCategory, SearchResult[]>();

		// Initialize with empty arrays for all categories
		Object.values(SearchResultCategory).forEach((category) => {
			grouped.set(category as SearchResultCategory, []);
		});

		// Add results to their respective categories
		results.forEach((result) => {
			const categoryResults = grouped.get(result.category) || [];
			categoryResults.push(result);
			grouped.set(result.category, categoryResults);
		});

		// Filter out empty categories
		return new Map(
			[...grouped.entries()].filter(([, categoryResults]) => categoryResults.length > 0)
		);
	}

	// Get human-readable category name
	function getCategoryName(category: SearchResultCategory): string {
		switch (category) {
			case SearchResultCategory.APPLICATION:
				return 'Applications';
			case SearchResultCategory.FILE:
				return 'Files';
			case SearchResultCategory.WEBSITE:
				return 'Websites';
			case SearchResultCategory.FOLDER:
				return 'Folders';
			case SearchResultCategory.DRIVE:
				return 'Cloud Storage';
			case SearchResultCategory.OTHER:
				return 'Other';
			default:
				return 'Results';
		}
	}

	// Reset selected index when results change
	$: if (results) {
		selectedIndex = results.length > 0 ? 0 : -1;
		resultElements = [];
	}

	// Bind event listener for keyboard navigation
	onMount(() => {
		window.addEventListener('keydown', handleKeyDown);
		return () => {
			window.removeEventListener('keydown', handleKeyDown);
		};
	});

	// Update result elements array after render
	afterUpdate(() => {
		resultElements = Array.from(document.querySelectorAll('[data-index]'));
	});
</script>

<div class="search-results-container max-h-[70vh] w-full overflow-y-auto" role="listbox">
	{#if loading}
		<div class="p-4 text-center text-gray-500">
			<div
				class="mx-auto mb-2 h-6 w-6 animate-spin rounded-full border-2 border-blue-500 border-t-transparent"
			></div>
			<p>Searching...</p>
		</div>
	{:else if results.length === 0}
		<div class="p-4 text-center text-gray-500">
			<p>{emptyMessage}</p>
		</div>
	{:else}
		{#each [...groupedResults.entries()] as [category, categoryResults]}
			<div class="category-group mb-4">
				<h3 class="px-3 py-1 text-xs font-semibold uppercase text-gray-500">
					{getCategoryName(category)} ({categoryResults.length})
				</h3>

				<div class="category-results">
					{#each categoryResults as result, i}
						{@const globalIndex = results.findIndex((r) => r.id === result.id)}
						<SearchResultItem
							{result}
							selected={selectedIndex === globalIndex}
							index={globalIndex}
						/>
					{/each}
				</div>
			</div>
		{/each}
	{/if}
</div>
