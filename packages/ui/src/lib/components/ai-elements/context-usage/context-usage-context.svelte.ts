import { getContext, setContext } from 'svelte';

const CONTEXT_USAGE_KEY = Symbol.for('context-usage');

export interface LanguageModelUsage {
	inputTokens?: number;
	outputTokens?: number;
	reasoningTokens?: number;
	cachedInputTokens?: number;
}

export type ModelId = string;

export class ContextUsageState {
	#usedTokens = $state(0);
	#maxTokens = $state(0);
	#usage = $state<LanguageModelUsage | undefined>(undefined);
	#modelId = $state<ModelId | undefined>(undefined);

	constructor(options: {
		usedTokens: number;
		maxTokens: number;
		usage?: LanguageModelUsage;
		modelId?: ModelId;
	}) {
		this.#usedTokens = options.usedTokens;
		this.#maxTokens = options.maxTokens;
		this.#usage = options.usage;
		this.#modelId = options.modelId;
	}

	get usedTokens() {
		return this.#usedTokens;
	}

	set usedTokens(value: number) {
		this.#usedTokens = value;
	}

	get maxTokens() {
		return this.#maxTokens;
	}

	set maxTokens(value: number) {
		this.#maxTokens = value;
	}

	get usage() {
		return this.#usage;
	}

	set usage(value: LanguageModelUsage | undefined) {
		this.#usage = value;
	}

	get modelId() {
		return this.#modelId;
	}

	set modelId(value: ModelId | undefined) {
		this.#modelId = value;
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
