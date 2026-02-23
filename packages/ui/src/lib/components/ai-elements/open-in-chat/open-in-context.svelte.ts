import { getContext, setContext } from 'svelte';

class OpenInState {
	query = $state('');

	constructor(query: string) {
		this.query = query;
	}
}

const SYMBOL_KEY = 'ai-open-in';

export function setOpenInContext(query: string): OpenInState {
	const state = new OpenInState(query);
	setContext(Symbol.for(SYMBOL_KEY), state);
	return state;
}

export function getOpenInContext(): OpenInState {
	const context = getContext<OpenInState>(Symbol.for(SYMBOL_KEY));
	if (!context) {
		throw new Error('OpenIn components must be used within an OpenIn provider');
	}
	return context;
}

export const providers = {
	chatgpt: {
		title: 'Open in ChatGPT',
		createUrl: (prompt: string) =>
			`https://chatgpt.com/?${new URLSearchParams({ hints: 'search', prompt })}`,
	},
	claude: {
		title: 'Open in Claude',
		createUrl: (q: string) => `https://claude.ai/new?${new URLSearchParams({ q })}`,
	},
	scira: {
		title: 'Open in Scira',
		createUrl: (q: string) => `https://scira.ai/?${new URLSearchParams({ q })}`,
	},
	t3: {
		title: 'Open in T3 Chat',
		createUrl: (q: string) => `https://t3.chat/new?${new URLSearchParams({ q })}`,
	},
	v0: {
		title: 'Open in v0',
		createUrl: (q: string) => `https://v0.app?${new URLSearchParams({ q })}`,
	},
	github: {
		title: 'Open in GitHub',
		createUrl: (url: string) => url,
	},
} as const;
