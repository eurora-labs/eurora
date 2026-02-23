import { getContext, setContext } from 'svelte';

const CHAIN_OF_THOUGHT_KEY = Symbol.for('chain-of-thought');

export class ChainOfThoughtState {
	#isOpen = $state(false);
	#isStreaming = $state(false);

	constructor(options: { isOpen?: boolean; isStreaming?: boolean }) {
		this.#isOpen = options.isOpen ?? false;
		this.#isStreaming = options.isStreaming ?? false;
	}

	get isOpen() {
		return this.#isOpen;
	}

	set isOpen(value: boolean) {
		this.#isOpen = value;
	}

	get isStreaming() {
		return this.#isStreaming;
	}

	set isStreaming(value: boolean) {
		this.#isStreaming = value;
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
