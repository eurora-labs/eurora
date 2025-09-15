import type { SveltePMExtension } from '@eurora/prosemirror-core/index';

/**
 * Type definition for extension creator functions
 */
export type ExtensionCreator = () => SveltePMExtension;

/**
 * ExtensionFactory for managing ProseMirror extensions
 */
export class ExtensionFactory {
	private registry: Map<string, ExtensionCreator> = new Map();

	/**
	 * Register an extension creator by its ID
	 */
	register(id: string, creator: ExtensionCreator): void {
		if (this.registry.has(id)) {
			console.warn(`Extension with ID ${id} is already registered. It will be overwritten.`);
		}
		this.registry.set(id, creator);
	}

	/**
	 * Get an extension instance by ID
	 */
	getExtension(id: string): SveltePMExtension {
		const creator = this.registry.get(id);
		if (!creator) {
			throw new Error(`Extension with ID ${id} not found`);
		}
		return creator();
	}

	/**
	 * Get all registered extensions as instances
	 */
	getExtensions(): SveltePMExtension[] {
		return Array.from(this.registry.values()).map((creator) => creator());
	}

	/**
	 * Check if an extension is registered
	 */
	hasExtension(id: string): boolean {
		return this.registry.has(id);
	}

	/**
	 * Get the IDs of all registered extensions
	 */
	getExtensionIds(): string[] {
		return Array.from(this.registry.keys());
	}

	/**
	 * Unregister an extension by ID
	 */
	unregister(id: string): boolean {
		return this.registry.delete(id);
	}
}

/**
 * Singleton instance of the extension factory
 */
export const extensionFactory = new ExtensionFactory();

/**
 * Re-export types for convenience
 */
export type { SveltePMExtension };
