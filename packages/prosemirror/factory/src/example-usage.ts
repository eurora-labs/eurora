import { extensionFactory } from './index.js';
import type { SveltePMExtension } from '@eurora/prosemirror-core';
import { getExtensionsByNamePattern, hasRequiredExtensions } from './utils.js';

// Ensure extensions are registered by importing the register-extensions module
import './register-extensions.js';

/**
 * Example: Get all extensions
 * This method returns all registered extensions as instances
 */
export function getAllExtensions(): SveltePMExtension[] {
	return extensionFactory.getExtensions();
}

/**
 * Example: Get specific extensions by ID
 * This method returns only the extensions with the specified IDs
 */
export function getSpecificExtensions(ids: string[]): SveltePMExtension[] {
	return ids
		.map((id) => extensionFactory.getExtension(id))
		.filter((ext): ext is SveltePMExtension => ext !== undefined);
}

/**
 * Example: Use with launcher search query
 * This demonstrates how to use the factory in the +page.svelte file
 */
export function createSearchQuery(text: string = ''): {
	text: string;
	extensions: SveltePMExtension[];
} {
	return {
		text,
		extensions: extensionFactory.getExtensions(),
	};
}

/**
 * Example: Check for required extensions before initializing editor
 * This pattern can be used to ensure all necessary extensions are available
 */
export function initializeEditorWithExtensions(editorElement: HTMLElement): void {
	// Define required extension IDs
	const requiredExtensions = [
		'9370B14D-B61C-4CE2-BDE7-B18684E8731A', // video
		'D8215655-A880-4B0F-8EFA-0B6B447F8AF3', // transcript
	];

	// Check if required extensions are available
	if (!hasRequiredExtensions(requiredExtensions)) {
		console.error('Not all required extensions are registered');
		// Handle missing extensions, e.g., show error message
		return;
	}

	// Get required extensions
	const extensions = getSpecificExtensions(requiredExtensions);

	// Initialize editor with extensions
	// This is just a placeholder - actual editor initialization would depend on your editor implementation
	console.log(`Initializing editor with ${extensions.length} extensions`);

	// In real usage, this would call your editor's initialization code
	// editor.init(extensions);
}

/**
 * Example: Get media-related extensions
 * This demonstrates using the pattern matching utility
 */
export function getMediaExtensions(): SveltePMExtension[] {
	return getExtensionsByNamePattern(/video|audio|media/i);
}

/**
 * Example: Runtime extension registration
 * This demonstrates how to register a new extension at runtime
 */
export function registerCustomExtension(id: string, creator: () => SveltePMExtension): void {
	// Register the extension
	extensionFactory.register(id, creator);

	// Log available extensions after registration
	console.log(
		`Extension ${id} registered. Available extensions:`,
		extensionFactory.getExtensionIds(),
	);
}
