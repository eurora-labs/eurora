import { extensionFactory, type SveltePMExtension } from './factory.js';

/**
 * Get extensions by name pattern
 * @param pattern Regular expression to match against extension names
 * @returns Array of extensions with matching names
 */
export function getExtensionsByNamePattern(pattern: RegExp): SveltePMExtension[] {
	return extensionFactory.getExtensions().filter((ext) => pattern.test(ext.name));
}

/**
 * Get extensions by node type
 * @param nodeType Node type to search for
 * @returns Array of extensions that provide the specified node type
 */
export function getExtensionsByNodeType(nodeType: string): SveltePMExtension[] {
	return extensionFactory
		.getExtensions()
		.filter((ext) => ext.svelteNodes && Object.keys(ext.svelteNodes).includes(nodeType));
}

/**
 * Extract schema configuration from extensions
 * @param extensions Array of extensions to extract schema from
 * @returns Object containing nodes and marks for schema creation
 */
export function createSchemaConfig(extensions: SveltePMExtension[]) {
	const nodes: Record<string, any> = {};
	const marks: Record<string, any> = {};

	for (const ext of extensions) {
		if (ext.svelteNodes) {
			for (const [name, node] of Object.entries(ext.svelteNodes)) {
				nodes[name] = node.schema;
			}
		}

		if (ext.marks) {
			for (const [name, mark] of Object.entries(ext.marks)) {
				if (mark.schema) {
					marks[name] = mark.schema;
				}
			}
		}
	}

	return { nodes, marks };
}

/**
 * Group extensions by category
 * This allows organizing extensions in the UI or other contexts
 * @returns Record mapping categories to arrays of extensions
 */
export function groupExtensionsByCategory(): Record<string, SveltePMExtension[]> {
	const result: Record<string, SveltePMExtension[]> = {};
	const extensions = extensionFactory.getExtensions();

	// Simple categorization by name pattern
	// This can be enhanced based on extension metadata if available
	for (const ext of extensions) {
		let category = 'other';

		if (ext.name.includes('video')) {
			category = 'media';
		} else if (ext.name.includes('transcript')) {
			category = 'document';
		}

		if (!result[category]) {
			result[category] = [];
		}

		result[category].push(ext);
	}

	return result;
}

/**
 * Check if all required extensions are registered
 * @param requiredIds Array of extension IDs that are required
 * @returns True if all required extensions are registered, false otherwise
 */
export function hasRequiredExtensions(requiredIds: string[]): boolean {
	const registeredIds = extensionFactory.getExtensionIds();
	return requiredIds.every((id) => registeredIds.includes(id));
}
