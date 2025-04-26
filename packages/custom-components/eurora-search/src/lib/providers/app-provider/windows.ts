import type { SearchProvider } from '../../types/search-provider';
import type { SearchResult } from '../../types/search-result';
import { SearchResultCategory } from '../../types/search-result';
import { searchWindowsApps, launchApplication } from '../../utils/tauri';
import { nanoid } from 'nanoid';

/**
 * Windows application search provider
 * Searches for applications in the Start Menu and registry
 */
export class WindowsAppProvider implements SearchProvider {
	id = 'windows-apps';
	name = 'Windows Applications';
	categories = [SearchResultCategory.APPLICATION];
	priority = 100;

	/**
	 * Initialize the provider - test connectivity
	 */
	async initialize(): Promise<void> {
		console.log('Initializing WindowsAppProvider...');
		try {
			// Test the connection by searching for a common term
			const testResults = await searchWindowsApps('test');
			console.log(
				`WindowsAppProvider initialization successful, found ${testResults.length} test results`
			);
		} catch (error) {
			console.error('Error initializing WindowsAppProvider:', error);
			throw new Error(`Failed to initialize Windows app provider: ${error}`);
		}
	}

	/**
	 * Search for Windows applications
	 */
	async search(query: string): Promise<SearchResult[]> {
		console.log(`WindowsAppProvider.search("${query}") called`);
		if (!query.trim()) return [];

		try {
			// Search for applications via Tauri
			console.log('Calling searchWindowsApps Tauri command...');
			const apps = await searchWindowsApps(query);
			console.log(`Received ${apps.length} results from Windows backend`);

			// Convert AppInfo to SearchResult
			return apps.map((app) => ({
				id: nanoid(),
				title: app.name,
				description: app.description || `Launch ${app.name}`,
				icon: app.icon,
				category: SearchResultCategory.APPLICATION,
				source: this.name,
				action: async () => {
					console.log(`Launching Windows application: ${app.path}`);
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
			console.error('Error searching Windows apps:', error);
			// Don't throw the error, just return empty results to avoid breaking the UI
			return [];
		}
	}

	/**
	 * Check if this provider is available (only on Windows)
	 */
	async isAvailable(): Promise<boolean> {
		console.log('Checking if WindowsAppProvider is available...');
		try {
			// Test the search functionality with a simple query
			const testApps = await searchWindowsApps('test');
			const isAvailable = testApps.length > 0;
			console.log(
				`WindowsAppProvider availability: ${isAvailable} (found ${testApps.length} test apps)`
			);
			return isAvailable;
		} catch (error) {
			console.error('Error checking Windows app provider availability:', error);
			return false;
		}
	}
}
