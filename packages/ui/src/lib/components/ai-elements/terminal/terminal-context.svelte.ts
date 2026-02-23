import { getContext, setContext } from 'svelte';

const TERMINAL_CONTEXT_KEY = Symbol.for('ai-terminal');

export class TerminalState {
	#output = $state('');
	#isStreaming = $state(false);
	#autoScroll = $state(true);
	#onClear: (() => void) | undefined;

	constructor(options: {
		output?: string;
		isStreaming?: boolean;
		autoScroll?: boolean;
		onClear?: () => void;
	}) {
		this.#output = options.output ?? '';
		this.#isStreaming = options.isStreaming ?? false;
		this.#autoScroll = options.autoScroll ?? true;
		this.#onClear = options.onClear;
	}

	get output() {
		return this.#output;
	}

	set output(value: string) {
		this.#output = value;
	}

	get isStreaming() {
		return this.#isStreaming;
	}

	set isStreaming(value: boolean) {
		this.#isStreaming = value;
	}

	get autoScroll() {
		return this.#autoScroll;
	}

	set autoScroll(value: boolean) {
		this.#autoScroll = value;
	}

	get onClear() {
		return this.#onClear;
	}

	set onClear(value: (() => void) | undefined) {
		this.#onClear = value;
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
