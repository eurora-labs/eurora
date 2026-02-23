import { getContext, setContext } from 'svelte';

const REASONING_CONTEXT_KEY = Symbol.for('reasoning-context');

export class ReasoningState {
	#isStreaming = $state(false);
	#isOpen = $state(false);
	#duration = $state<number | undefined>(undefined);

	constructor(options: { isStreaming?: boolean; isOpen?: boolean; duration?: number }) {
		this.#isStreaming = options.isStreaming ?? false;
		this.#isOpen = options.isOpen ?? false;
		this.#duration = options.duration;
	}

	get isStreaming() {
		return this.#isStreaming;
	}

	set isStreaming(value: boolean) {
		this.#isStreaming = value;
	}

	get isOpen() {
		return this.#isOpen;
	}

	set isOpen(value: boolean) {
		this.#isOpen = value;
	}

	get duration() {
		return this.#duration;
	}

	set duration(value: number | undefined) {
		this.#duration = value;
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
