import { extensionFactory } from './index.js';

// Import extensions
import { videoExtension, videoExtensionID } from '@eurora/ext-video';
import { transcriptExtension, transcriptExtensionID } from '@eurora/ext-transcript';
import { youtubeVideoExtension, youtubeVideoExtensionID } from '@eurora/ext-youtube-video';

/**
 * Register all known core extensions
 * This function registers built-in extensions.
 * Additional extensions can be registered by applications as needed.
 */
export function registerCoreExtensions(): void {
	extensionFactory.register(videoExtensionID, videoExtension);
	extensionFactory.register(transcriptExtensionID, transcriptExtension);
	extensionFactory.register(youtubeVideoExtensionID, youtubeVideoExtension);
}

// Option 1: Auto-register extensions when this module is imported
// Uncomment the line below to automatically register extensions when this module is imported
// registerCoreExtensions();

/**
 * Alternative approach: Lazy loading extensions
 * This provides an async way to load extensions only when needed
 */
// export async function lazyRegisterCoreExtensions(): Promise<void> {
// 	try {
// 		const videoModule = await import('@eurora/ext-video');
// 		extensionFactory.register(videoExtensionID, videoModule.videoExtension);

// 		const transcriptModule = await import('@eurora/ext-transcript');
// 		extensionFactory.register(transcriptExtensionID, transcriptModule.transcriptExtension);
// 	} catch (error) {
// 		console.error('Failed to register core extensions:', error);
// 		throw error;
// 	}
// }
