import { getContext, setContext } from 'svelte';

const REASONING_CONTEXT_KEY = Symbol.for('reasoning-context');

export interface ReasoningStateOptions {
	isStreaming?: () => boolean;
	isOpen?: () => boolean;
	duration?: () => number | undefined;
}

export class ReasoningState {
	readonly #opts: ReasoningStateOptions;

	constructor(opts: ReasoningStateOptions) {
		this.#opts = opts;
	}

	get isStreaming(): boolean {
		return this.#opts.isStreaming?.() ?? false;
	}

	get isOpen(): boolean {
		return this.#opts.isOpen?.() ?? false;
	}

	get duration(): number | undefined {
		return this.#opts.duration?.();
	}
}

export function setReasoningContext(state: ReasoningState) {
	setContext(REASONING_CONTEXT_KEY, state);
}

export function getReasoningContext(): ReasoningState {
	const context = getContext<ReasoningState | undefined>(REASONING_CONTEXT_KEY);
	if (!context) {
		throw new Error('Reasoning components must be used within Reasoning');
	}
	return context;
}
