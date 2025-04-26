import type { SearchProvider } from '../types/search-provider';
import type { SearchResult } from '../types/search-result';
import { SearchResultCategory } from '../types/search-result';
import { createAppProvider } from '../providers/app-provider';
import { detectOperatingSystem, OperatingSystem, isTauri } from '../utils/platform';

/**
 * Search engine that manages and coordinates all search providers
 */
export class SearchEngine {
	private providers: Map<string, SearchProvider> = new Map();
	private initialized = false;

	/**
	 * Initialize the search engine and all active providers
	 */
	async initialize(): Promise<void> {
		if (this.initialized) return;

		console.log('Initializing SearchEngine...');
		console.log(`Detected operating system: ${detectOperatingSystem()}`);
		console.log(`Running in Tauri: ${isTauri()}`);

		// Register the app provider based on the current platform
		try {
			console.log('Creating app provider...');
			const appProvider = createAppProvider();
			console.log(`Created app provider: ${appProvider.id}`);

			let isAvailable = true;
			if (appProvider.isAvailable) {
				console.log('Checking if app provider is available...');
				try {
					isAvailable = await appProvider.isAvailable();
					console.log(`App provider availability: ${isAvailable}`);
				} catch (error) {
					console.error('Error checking app provider availability:', error);
					isAvailable = false;
				}
			}

			if (isAvailable) {
				this.registerProvider(appProvider);
				console.log(`Registered app provider: ${appProvider.id}`);

				if (appProvider.initialize) {
					console.log('Initializing app provider...');
					try {
						await appProvider.initialize();
						console.log('App provider initialized successfully');
					} catch (error) {
						console.error('Error initializing app provider:', error);
					}
				}
			} else {
				console.warn(`App provider ${appProvider.id} is not available on this platform`);
			}
		} catch (error) {
			console.error('Error initializing app provider:', error);
		}

		// In the future, add more providers here:
		// - Google Drive provider
		// - Local files provider
		// - Browser history provider

		// If no providers were registered, add a dummy provider for testing
		if (this.providers.size === 0) {
			console.warn('No providers registered, adding dummy provider for testing');
			this.registerProvider({
				id: 'dummy-provider',
				name: 'Dummy Provider',
				categories: [SearchResultCategory.APPLICATION],
				priority: 0,
				search: async (query: string) => {
					console.log('Dummy provider searching for:', query);
					return [
						{
							id: 'dummy-1',
							title: `Dummy Result for "${query}"`,
							description: 'This is a dummy result for testing',
							category: SearchResultCategory.APPLICATION,
							source: 'Dummy Provider',
							action: () => console.log('Dummy action executed')
						}
					];
				}
			});
		}

		this.initialized = true;
		console.log(`SearchEngine initialized with ${this.providers.size} providers`);
	}

	/**
	 * Register a new search provider
	 */
	registerProvider(provider: SearchProvider): void {
		this.providers.set(provider.id, provider);
	}

	/**
	 * Remove a search provider
	 */
	unregisterProvider(providerId: string): void {
		this.providers.delete(providerId);
	}

	/**
	 * Get all registered providers
	 */
	getProviders(): SearchProvider[] {
		return Array.from(this.providers.values());
	}

	/**
	 * Get providers that can search for a specific category
	 */
	getProvidersByCategory(category: SearchResultCategory): SearchProvider[] {
		return this.getProviders().filter((provider) => provider.categories.includes(category));
	}

	/**
	 * Execute a search across all providers
	 */
	async search(query: string): Promise<SearchResult[]> {
		console.log(`SearchEngine.search("${query}") called`);

		if (!this.initialized) {
			console.log('SearchEngine not initialized, initializing now...');
			await this.initialize();
		}

		if (!query) {
			console.log('Empty query, returning empty results');
			return [];
		}

		console.log(`Searching with ${this.providers.size} providers`);

		// Get all providers, sorted by priority
		const providers = this.getProviders().sort((a, b) => (b.priority || 0) - (a.priority || 0));

		console.log(`Sorted providers: ${providers.map((p) => p.id).join(', ')}`);

		// Execute search on all providers in parallel
		const resultsPromises = providers.map(async (provider) => {
			try {
				console.log(`Searching with provider: ${provider.id}`);
				const results = await provider.search(query);
				console.log(`Provider ${provider.id} returned ${results.length} results`);
				return results;
			} catch (error) {
				console.error(`Error from provider ${provider.id}:`, error);
				return [];
			}
		});

		// Wait for all providers to complete
		const resultsArrays = await Promise.all(resultsPromises);

		// Flatten and return results
		const allResults = resultsArrays.flat();
		console.log(`Total results: ${allResults.length}`);
		return allResults;
	}

	/**
	 * Clean up resources used by providers
	 */
	async dispose(): Promise<void> {
		console.log('Disposing SearchEngine...');
		for (const provider of this.getProviders()) {
			if (provider.dispose) {
				try {
					console.log(`Disposing provider: ${provider.id}`);
					await provider.dispose();
				} catch (error) {
					console.error(`Error disposing provider ${provider.id}:`, error);
				}
			}
		}

		this.providers.clear();
		this.initialized = false;
		console.log('SearchEngine disposed');
	}
}
