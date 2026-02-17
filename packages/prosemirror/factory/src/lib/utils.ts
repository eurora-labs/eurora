import { extensionFactory, type SveltePMExtension } from '$lib/factory.js';

export function getExtensionsByNamePattern(pattern: RegExp): SveltePMExtension[] {
	return extensionFactory.getExtensions().filter((ext) => pattern.test(ext.name));
}

export function getExtensionsByNodeType(nodeType: string): SveltePMExtension[] {
	return extensionFactory
		.getExtensions()
		.filter((ext) => ext.svelteNodes && Object.keys(ext.svelteNodes).includes(nodeType));
}

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

export function groupExtensionsByCategory(): Record<string, SveltePMExtension[]> {
	const result: Record<string, SveltePMExtension[]> = {};
	const extensions = extensionFactory.getExtensions();

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

export function hasRequiredExtensions(requiredIds: string[]): boolean {
	const registeredIds = extensionFactory.getExtensionIds();
	return requiredIds.every((id) => registeredIds.includes(id));
}
