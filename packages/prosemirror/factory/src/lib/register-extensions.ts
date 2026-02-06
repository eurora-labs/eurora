import { extensionFactory } from '$lib/factory.js';

// Import extensions
import { articleExtension, articleExtensionID } from '@eurora/prosemirror-extensions/article/index';
import { twitterExtension, twitterExtensionID } from '@eurora/prosemirror-extensions/twitter/index';
import {
	youtubeVideoExtension,
	youtubeVideoExtensionID,
} from '@eurora/prosemirror-extensions/youtube/index';

/**
 * Register all known core extensions
 * This function registers built-in extensions.
 * Additional extensions can be registered by applications as needed.
 */
export function registerCoreExtensions(): void {
	extensionFactory.register(articleExtensionID, articleExtension);
	extensionFactory.register(youtubeVideoExtensionID, youtubeVideoExtension);
	extensionFactory.register(twitterExtensionID, twitterExtension);
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
