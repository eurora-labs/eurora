/**
 * Content Script Strategy Interface
 * Defines the contract for all content script strategies
 */
export interface ContentScriptStrategy {
	/**
	 * Check if this strategy can handle the given URL
	 * @param url The URL to check
	 * @returns True if this strategy can handle the URL
	 */
	canHandle(url: string): boolean;

	/**
	 * Initialize the content script for the given tab
	 * @param tabId The ID of the tab
	 * @param url The URL of the tab
	 */
	initialize(tabId: number, url: string): void;
}

/**
 * Content Script Strategy Context
 * Manages the available strategies and delegates to the appropriate one
 */
export class ContentScriptContext {
	private strategies: ContentScriptStrategy[] = [];
	private defaultStrategy: ContentScriptStrategy;

	/**
	 * Register a strategy with the context
	 * @param strategy The strategy to register
	 * @param isDefault Whether this is the default strategy
	 */
	registerStrategy(strategy: ContentScriptStrategy, isDefault: boolean = false): void {
		this.strategies.push(strategy);

		if (isDefault) {
			this.defaultStrategy = strategy;
		}
	}

	/**
	 * Process a tab update with the appropriate strategy
	 * @param tabId The ID of the tab
	 * @param url The URL of the tab
	 * @returns True if a strategy was found and executed
	 */
	processTab(tabId: number, url: string): boolean {
		// Find the first strategy that can handle this URL
		const strategy = this.strategies.find((s) => s.canHandle(url));

		if (strategy) {
			strategy.initialize(tabId, url);
			return true;
		} else if (this.defaultStrategy) {
			// Use the default strategy if no other strategy can handle the URL
			this.defaultStrategy.initialize(tabId, url);
			return true;
		}

		return false;
	}
}
