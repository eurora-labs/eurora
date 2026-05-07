import { getContext, setContext } from 'svelte';

const WEB_PREVIEW_CONTEXT_KEY = Symbol.for('web-preview-context');

export interface WebPreviewContextOptions {
	initialUrl?: string;
	onUrlChange?: () => ((url: string) => void) | undefined;
}

export class WebPreviewContext {
	readonly #onUrlChange: () => ((url: string) => void) | undefined;
	#url = $state('');
	#consoleOpen = $state(false);

	constructor(opts: WebPreviewContextOptions = {}) {
		this.#url = opts.initialUrl ?? '';
		this.#onUrlChange = opts.onUrlChange ?? (() => undefined);
	}

	get url() {
		return this.#url;
	}

	set url(value: string) {
		this.#url = value;
		this.#onUrlChange()?.(value);
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
