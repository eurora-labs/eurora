import { getContext, setContext } from 'svelte';

const CHAIN_OF_THOUGHT_KEY = Symbol.for('chain-of-thought');

export interface ChainOfThoughtStateOptions {
	isOpen: () => boolean;
	setOpen: (value: boolean) => void;
	isStreaming?: () => boolean;
}

export class ChainOfThoughtState {
	readonly #opts: ChainOfThoughtStateOptions;

	constructor(opts: ChainOfThoughtStateOptions) {
		this.#opts = opts;
	}

	get isOpen(): boolean {
		return this.#opts.isOpen();
	}

	set isOpen(value: boolean) {
		this.#opts.setOpen(value);
	}

	get isStreaming(): boolean {
		return this.#opts.isStreaming?.() ?? false;
	}
}

export function setChainOfThoughtContext(state: ChainOfThoughtState) {
	setContext(CHAIN_OF_THOUGHT_KEY, state);
}

export function getChainOfThoughtContext(): ChainOfThoughtState {
	const context = getContext<ChainOfThoughtState | undefined>(CHAIN_OF_THOUGHT_KEY);
	if (!context) {
		throw new Error('ChainOfThought components must be used within ChainOfThought');
	}
	return context;
}
