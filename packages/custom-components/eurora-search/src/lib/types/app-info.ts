/**
 * Interface for application information
 * Used by the application search providers
 */
export interface AppInfo {
	/** Application name */
	name: string;

	/** Path to executable */
	path: string;

	/** Optional application description */
	description?: string;

	/** Optional path or base64 data for the application icon */
	icon?: string;

	/** Additional platform-specific metadata */
	metadata?: Record<string, any>;
}
