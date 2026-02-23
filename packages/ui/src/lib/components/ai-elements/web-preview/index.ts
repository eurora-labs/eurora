import Root from './WebPreview.svelte';
import Navigation from './WebPreviewNavigation.svelte';
import Url from './WebPreviewUrl.svelte';
import Body from './WebPreviewBody.svelte';
import Console from './WebPreviewConsole.svelte';

export {
	Root,
	Navigation,
	Url,
	Body,
	Console,
	//
	Root as WebPreview,
	Navigation as WebPreviewNavigation,
	Url as WebPreviewUrl,
	Body as WebPreviewBody,
	Console as WebPreviewConsole,
};

export * from './web-preview-context.svelte.js';
export type { ConsoleLog } from './WebPreviewConsole.svelte';
