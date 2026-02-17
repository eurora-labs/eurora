import { extensionFactory } from '$lib/factory.js';

import { articleExtension, articleExtensionID } from '@eurora/prosemirror-extensions/article/index';
import { twitterExtension, twitterExtensionID } from '@eurora/prosemirror-extensions/twitter/index';
import {
	youtubeVideoExtension,
	youtubeVideoExtensionID,
} from '@eurora/prosemirror-extensions/youtube/index';

export function registerCoreExtensions(): void {
	extensionFactory.register(articleExtensionID, articleExtension);
	extensionFactory.register(youtubeVideoExtensionID, youtubeVideoExtension);
	extensionFactory.register(twitterExtensionID, twitterExtension);
}
