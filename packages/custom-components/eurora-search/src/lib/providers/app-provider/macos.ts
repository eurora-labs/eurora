import type { SearchProvider } from '../../types/search-provider';
import type { SearchResult } from '../../types/search-result';
import { SearchResultCategory } from '../../types/search-result';
import { searchMacOsApps, launchApplication } from '../../utils/tauri';
import { nanoid } from 'nanoid';

/**
 * macOS application search provider
 * Searches for applications in the Applications folder and using Spotlight
 */
export class MacOsAppProvider implements SearchProvider {
	id = 'macos-apps';
	name = 'macOS Applications';
	categories = [SearchResultCategory.APPLICATION];
	priority = 100;

	/**
	 * Initialize the provider - test connectivity
	 */
	async initialize(): Promise<void> {
		console.log('Initializing MacOsAppProvider...');
		try {
			// Test the connection by searching for a common term
			const testResults = await searchMacOsApps('test');
			console.log(
				`MacOsAppProvider initialization successful, found ${testResults.length} test results`
			);
		} catch (error) {
			console.error('Error initializing MacOsAppProvider:', error);
			throw new Error(`Failed to initialize macOS app provider: ${error}`);
		}
	}

	/**
	 * Search for macOS applications
	 */
	async search(query: string): Promise<SearchResult[]> {
		console.log(`MacOsAppProvider.search("${query}") called`);
		if (!query.trim()) return [];

		try {
			// Search for applications via Tauri
			console.log('Calling searchMacOsApps Tauri command...');
			const apps = await searchMacOsApps(query);
			console.log(`Received ${apps.length} results from macOS backend`);

			// Convert AppInfo to SearchResult
			return apps.map((app) => ({
				id: nanoid(),
				title: app.name,
				description: app.description || `Launch ${app.name}`,
				icon: app.icon,
				category: SearchResultCategory.APPLICATION,
				source: this.name,
				action: async () => {
					console.log(`Launching macOS application: ${app.path}`);
					try {
						await launchApplication(app.path);
						console.log(`Successfully launched: ${app.path}`);
					} catch (error) {
						console.error(`Failed to launch application ${app.path}:`, error);
					}
				},
				data: {
					path: app.path,
					...app.metadata
				}
			}));
		} catch (error) {
			console.error('Error searching macOS apps:', error);
			// Don't throw the error, just return empty results to avoid breaking the UI
			return [];
		}
	}

	/**
	 * Check if this provider is available (only on macOS)
	 */
	async isAvailable(): Promise<boolean> {
		console.log('Checking if MacOsAppProvider is available...');
		try {
			// Test the search functionality with a simple query
			const testApps = await searchMacOsApps('test');
			const isAvailable = testApps.length > 0;
			console.log(
				`MacOsAppProvider availability: ${isAvailable} (found ${testApps.length} test apps)`
			);
			return isAvailable;
		} catch (error) {
			console.error('Error checking macOS app provider availability:', error);
			return false;
		}
	}
}
