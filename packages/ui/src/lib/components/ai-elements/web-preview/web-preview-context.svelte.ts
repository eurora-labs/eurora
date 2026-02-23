import { getContext, setContext } from 'svelte';

const WEB_PREVIEW_CONTEXT_KEY = Symbol.for('web-preview-context');

export class WebPreviewContext {
	#url = $state('');
	#consoleOpen = $state(false);
	#onUrlChange: ((url: string) => void) | undefined;

	constructor(
		options: {
			url?: string;
			consoleOpen?: boolean;
			onUrlChange?: (url: string) => void;
		} = {},
	) {
		this.#url = options.url ?? '';
		this.#consoleOpen = options.consoleOpen ?? false;
		this.#onUrlChange = options.onUrlChange;
	}

	get url() {
		return this.#url;
	}

	set url(value: string) {
		this.#url = value;
		this.#onUrlChange?.(value);
	}

	get consoleOpen() {
		return this.#consoleOpen;
	}

	set consoleOpen(value: boolean) {
		this.#consoleOpen = value;
	}
}

export function setWebPreviewContext(context: WebPreviewContext) {
	setContext(WEB_PREVIEW_CONTEXT_KEY, context);
}

export function getWebPreviewContext(): WebPreviewContext {
	const context = getContext<WebPreviewContext | undefined>(WEB_PREVIEW_CONTEXT_KEY);
	if (!context) {
		throw new Error('WebPreview components must be used within WebPreview');
	}
	return context;
}
