/**
 * Categories of search results
 */
export enum SearchResultCategory {
	APPLICATION = 'application',
	FILE = 'file',
	WEBSITE = 'website',
	FOLDER = 'folder',
	DRIVE = 'drive',
	OTHER = 'other'
}

/**
 * Common interface for all search results
 */
export interface SearchResult {
	id: string;
	title: string;
	description?: string;
	icon?: string;
	category: SearchResultCategory;
	source: string;
	action: () => Promise<void> | void;
	data?: Record<string, any>;
}
