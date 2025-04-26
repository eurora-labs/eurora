import type { SearchResult, SearchResultCategory } from './search-result';

/**
 * Interface for search providers
 * Each provider can search for specific types of results
 */
export interface SearchProvider {
	/** Unique identifier for the provider */
	id: string;

	/** Display name of the provider */
	name: string;

	/** Categories of results this provider can return */
	categories: SearchResultCategory[];

	/** Main search function that returns results */
	search: (query: string) => Promise<SearchResult[]>;

	/** Optional initialization function */
	initialize?: () => Promise<void>;

	/** Optional cleanup function */
	dispose?: () => Promise<void>;

	/** Search priority (higher numbers are searched first) */
	priority?: number;

	/** Function to check if this provider is available on the current platform */
	isAvailable?: () => Promise<boolean>;
}
