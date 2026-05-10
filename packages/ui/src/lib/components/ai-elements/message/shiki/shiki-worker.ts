/// <reference lib="webworker" />

import {
	createHighlighterCore,
	type HighlighterCore,
	type ThemedToken,
	type LanguageRegistration,
} from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import githubDark from '@shikijs/themes/github-dark';
import githubLight from '@shikijs/themes/github-light';

import { bundledLanguages } from './languages.js';

export type ShikiWorkerRequest =
	| {
			type: 'highlight';
			id: number;
			code: string;
			lang: string;
			theme: string;
	  }
	| {
			type: 'warmup';
			langs: string[];
	  };

export interface ShikiHighlightResponse {
	type: 'highlight';
	id: number;
	tokens: ThemedToken[][];
}

const ctx = self as unknown as DedicatedWorkerGlobalScope;

const langLoaders = new Map<
	string,
	() => Promise<{ default: LanguageRegistration | LanguageRegistration[] }>
>();
for (const lang of bundledLanguages) {
	langLoaders.set(lang.id, lang.import);
	if (lang.aliases) {
		for (const alias of lang.aliases) langLoaders.set(alias, lang.import);
	}
}

let highlighterPromise: Promise<HighlighterCore> | null = null;
const langLoadState = new Map<string, Promise<boolean> | boolean>();

function getHighlighter(): Promise<HighlighterCore> {
	if (!highlighterPromise) {
		highlighterPromise = createHighlighterCore({
			themes: [githubDark, githubLight],
			langs: [],
			engine: createJavaScriptRegexEngine({ forgiving: true }),
		});
	}
	return highlighterPromise;
}

async function ensureLanguage(highlighter: HighlighterCore, lang: string): Promise<boolean> {
	const cached = langLoadState.get(lang);
	if (cached === true) return true;
	if (cached === false) return false;
	if (cached) return cached;

	const loader = langLoaders.get(lang);
	if (!loader) {
		langLoadState.set(lang, false);
		return false;
	}

	const promise = (async () => {
		try {
			const mod = await loader();
			await highlighter.loadLanguage(mod.default);
			langLoadState.set(lang, true);
			return true;
		} catch (err) {
			console.error(`[shiki-worker] failed to load language "${lang}":`, err);
			langLoadState.set(lang, false);
			return false;
		}
	})();

	langLoadState.set(lang, promise);
	return promise;
}

function plaintextTokens(code: string): ThemedToken[][] {
	const result: ThemedToken[][] = [];
	let offset = 0;
	for (const line of code.split('\n')) {
		result.push([{ content: line, offset }]);
		offset += line.length + 1; // account for the consumed '\n'
	}
	return result;
}

async function handleHighlight(req: Extract<ShikiWorkerRequest, { type: 'highlight' }>): Promise<ShikiHighlightResponse> {
	try {
		const highlighter = await getHighlighter();

		if (!highlighter.getLoadedThemes().includes(req.theme)) {
			return { type: 'highlight', id: req.id, tokens: plaintextTokens(req.code) };
		}

		const loaded = await ensureLanguage(highlighter, req.lang);
		if (!loaded) {
			return { type: 'highlight', id: req.id, tokens: plaintextTokens(req.code) };
		}

		const tokens = highlighter.codeToTokensBase(req.code, {
			lang: req.lang,
			theme: req.theme,
		});
		return { type: 'highlight', id: req.id, tokens };
	} catch (err) {
		console.error('[shiki-worker] highlight failed:', err);
		return { type: 'highlight', id: req.id, tokens: plaintextTokens(req.code) };
	}
}

async function handleWarmup(req: Extract<ShikiWorkerRequest, { type: 'warmup' }>): Promise<void> {
	try {
		const highlighter = await getHighlighter();
		await Promise.all(req.langs.map((lang) => ensureLanguage(highlighter, lang)));
	} catch (err) {
		console.warn('[shiki-worker] warmup failed:', err);
	}
}

ctx.addEventListener('message', (event: MessageEvent<ShikiWorkerRequest>) => {
	const req = event.data;
	if (!req || typeof req !== 'object') return;

	if (req.type === 'highlight') {
		void handleHighlight(req).then((res) => ctx.postMessage(res));
		return;
	}

	if (req.type === 'warmup') {
		void handleWarmup(req);
		return;
	}
});
