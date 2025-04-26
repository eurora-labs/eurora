<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import type { SearchResult } from '../types/search-result';
	import { SearchEngine } from '../services/search-engine';
	import SearchBar from './SearchBar.svelte';
	import SearchResults from './SearchResults.svelte';

	export let placeholder = 'Search for anything...';
	export let autofocus = true;
	export let emptyMessage = 'Type to search for apps, files, and more';

	// Search state
	let query = '';
	let results: SearchResult[] = [];
	let loading = false;
	let engine: SearchEngine;
	let searchTimeoutId: ReturnType<typeof setTimeout> | undefined;
	let error: string | null = null;

	// Initialize search engine on mount
	onMount(() => {
		console.log('Search component mounted, initializing engine...');
		try {
			engine = new SearchEngine();
			engine.initialize();
			console.log('Search engine initialized successfully');
		} catch (err) {
			console.error('Failed to initialize search engine:', err);
			error = 'Failed to initialize search engine. Please check the console for details.';
		}
	});

	// Clean up search engine on destroy
	onDestroy(async () => {
		if (engine) {
			await engine.dispose();
		}
	});

	// Handle search query
	async function handleSearch(searchQuery: string) {
		query = searchQuery;
		console.log('Searching for:', query);

		// Clear previous timeout
		if (searchTimeoutId) {
			clearTimeout(searchTimeoutId);
		}

		// Skip empty queries
		if (!query.trim()) {
			results = [];
			loading = false;
			return;
		}

		// Set loading state
		loading = true;

		try {
			// Perform search
			console.log('Executing search with query:', query);
			const searchResults = await engine.search(query);
			console.log('Search results:', searchResults);
			results = searchResults;
		} catch (err) {
			console.error('Search error:', err);
			error = `Search failed: ${err instanceof Error ? err.message : String(err)}`;
			results = [];
		} finally {
			loading = false;
		}
	}
</script>

<div class="search-container mx-auto w-full max-w-2xl p-2">
	<div class="mb-4">
		<SearchBar
			bind:value={query}
			{placeholder}
			{autofocus}
			on:search={(e) => handleSearch(e.detail)}
		/>
	</div>

	{#if error}
		<div class="error-message mb-3 rounded bg-red-100 p-3 text-red-800">
			{error}
		</div>
	{/if}

	<div class="search-results rounded bg-white shadow-lg">
		<SearchResults {results} {loading} emptyMessage={query ? 'No results found' : emptyMessage} />
	</div>
</div>
