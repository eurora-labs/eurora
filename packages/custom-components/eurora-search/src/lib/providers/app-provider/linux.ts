import type { SearchProvider } from '../../types/search-provider';
import type { SearchResult } from '../../types/search-result';
import { SearchResultCategory } from '../../types/search-result';
import { searchLinuxApps, launchApplication } from '../../utils/tauri';
import { nanoid } from 'nanoid';

/**
 * Linux application search provider
 * Searches for applications using .desktop files in standard locations
 */
export class LinuxAppProvider implements SearchProvider {
	id = 'linux-apps';
	name = 'Linux Applications';
	categories = [SearchResultCategory.APPLICATION];
	priority = 100;

	/**
	 * Initialize the provider - test connectivity
	 */
	async initialize(): Promise<void> {
		console.log('Initializing LinuxAppProvider...');
		try {
			// Test the connection by searching for a common term
			const testResults = await searchLinuxApps('');
			console.log(
				`LinuxAppProvider initialization successful, found ${testResults.length} test results`
			);
		} catch (error) {
			console.error('Error initializing LinuxAppProvider:', error);
			throw new Error(`Failed to initialize Linux app provider: ${error}`);
		}
	}

	/**
	 * Search for Linux applications
	 */
	async search(query: string): Promise<SearchResult[]> {
		console.log(`LinuxAppProvider.search("${query}") called`);
		if (!query.trim()) return [];

		try {
			// Search for applications via Tauri
			console.log('Calling searchLinuxApps Tauri command...');
			const apps = await searchLinuxApps(query);
			console.log(`Received ${apps.length} results from Linux backend`);

			// Convert AppInfo to SearchResult
			return apps.map((app) => ({
				id: nanoid(),
				title: app.name,
				description: app.description || `Launch ${app.name}`,
				icon: app.icon,
				category: SearchResultCategory.APPLICATION,
				source: this.name,
				action: async () => {
					console.log(`Launching Linux application: ${app.path}`);
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
			console.error('Error searching Linux apps:', error);
			// Don't throw the error, just return empty results to avoid breaking the UI
			return [];
		}
	}

	/**
	 * Check if this provider is available (only on Linux)
	 */
	async isAvailable(): Promise<boolean> {
		console.log('Checking if LinuxAppProvider is available...');
		try {
			// Test the search functionality with a simple query
			const testApps = await searchLinuxApps('');
			const isAvailable = testApps.length > 0;
			console.log(
				`LinuxAppProvider availability: ${isAvailable} (found ${testApps.length} test apps)`
			);
			return isAvailable;
		} catch (error) {
			console.error('Error checking Linux app provider availability:', error);
			return false;
		}
	}
}
