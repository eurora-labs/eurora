import { getContext, setContext } from 'svelte';

const TERMINAL_CONTEXT_KEY = Symbol.for('ai-terminal');

export interface TerminalStateOptions {
	output?: () => string;
	isStreaming?: () => boolean;
	autoScroll?: () => boolean;
	onClear?: () => (() => void) | undefined;
}

export class TerminalState {
	readonly #opts: TerminalStateOptions;

	constructor(opts: TerminalStateOptions) {
		this.#opts = opts;
	}

	get output(): string {
		return this.#opts.output?.() ?? '';
	}

	get isStreaming(): boolean {
		return this.#opts.isStreaming?.() ?? false;
	}

	get autoScroll(): boolean {
		return this.#opts.autoScroll?.() ?? true;
	}

	get onClear(): (() => void) | undefined {
		return this.#opts.onClear?.();
	}
}

export function setTerminalContext(state: TerminalState) {
	setContext(TERMINAL_CONTEXT_KEY, state);
}

export function getTerminalContext(): TerminalState {
	const context = getContext<TerminalState | undefined>(TERMINAL_CONTEXT_KEY);
	if (!context) {
		throw new Error('Terminal components must be used within Terminal');
	}
	return context;
}
