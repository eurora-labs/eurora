import { getContext, setContext } from 'svelte';

const CONTEXT_USAGE_KEY = Symbol.for('context-usage');

export interface LanguageModelUsage {
	inputTokens?: number;
	outputTokens?: number;
	reasoningTokens?: number;
	cachedInputTokens?: number;
}

export type ModelId = string;

export interface ContextUsageStateOptions {
	usedTokens: () => number;
	maxTokens: () => number;
	usage?: () => LanguageModelUsage | undefined;
	modelId?: () => ModelId | undefined;
}

export class ContextUsageState {
	readonly #opts: ContextUsageStateOptions;

	constructor(opts: ContextUsageStateOptions) {
		this.#opts = opts;
	}

	get usedTokens(): number {
		return this.#opts.usedTokens();
	}

	get maxTokens(): number {
		return this.#opts.maxTokens();
	}

	get usage(): LanguageModelUsage | undefined {
		return this.#opts.usage?.();
	}

	get modelId(): ModelId | undefined {
		return this.#opts.modelId?.();
	}
}

export function setContextUsageContext(state: ContextUsageState) {
	setContext(CONTEXT_USAGE_KEY, state);
}

export function getContextUsageContext(): ContextUsageState {
	const context = getContext<ContextUsageState | undefined>(CONTEXT_USAGE_KEY);
	if (!context) {
		throw new Error('ContextUsage components must be used within ContextUsage');
	}
	return context;
}
