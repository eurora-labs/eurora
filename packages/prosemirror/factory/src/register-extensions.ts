import { extensionFactory } from './index.js';

// Import extensions
import { videoExtension } from '@eurora/ext-video';
import { transcriptExtension } from '@eurora/ext-transcript';

/**
 * Register all known core extensions
 * This function registers built-in extensions.
 * Additional extensions can be registered by applications as needed.
 */
export function registerCoreExtensions(): void {
	// Register video extension
	const videoExt = videoExtension();
	extensionFactory.register(videoExt.id, videoExtension);

	// Register transcript extension
	const transcriptExt = transcriptExtension();
	extensionFactory.register(transcriptExt.id, transcriptExtension);

	// Additional extensions can be registered here as they are added to the system
}

// Option 1: Auto-register extensions when this module is imported
// Uncomment the line below to automatically register extensions when this module is imported
// registerCoreExtensions();

/**
 * Alternative approach: Lazy loading extensions
 * This provides an async way to load extensions only when needed
 */
export async function lazyRegisterCoreExtensions(): Promise<void> {
	try {
		// Video extension
		const videoModule = await import('@eurora/ext-video');
		const videoExt = videoModule.videoExtension();
		extensionFactory.register(videoExt.id, videoModule.videoExtension);

		// Transcript extension
		const transcriptModule = await import('@eurora/ext-transcript');
		const transcriptExt = transcriptModule.transcriptExtension();
		extensionFactory.register(transcriptExt.id, transcriptModule.transcriptExtension);

		// Additional extensions can be loaded here
	} catch (error) {
		console.error('Failed to register core extensions:', error);
		throw error;
	}
}
