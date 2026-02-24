import { getContext, setContext } from 'svelte';

const MESSAGE_BRANCH_CONTEXT_KEY = Symbol.for('message-branch-context');

export type MessageRole = 'user' | 'assistant' | 'system' | 'function' | 'data' | 'tool';

export class MessageBranchState {
	currentBranch = $state(0);
	totalBranches = $state(0);
	#onBranchChange?: (index: number) => void;

	constructor(defaultBranch: number = 0, onBranchChange?: (index: number) => void) {
		this.currentBranch = defaultBranch;
		this.#onBranchChange = onBranchChange;
	}

	goToPrevious() {
		const newBranch = this.currentBranch > 0 ? this.currentBranch - 1 : this.totalBranches - 1;
		this.currentBranch = newBranch;
		this.#onBranchChange?.(newBranch);
	}

	goToNext() {
		const newBranch = this.currentBranch < this.totalBranches - 1 ? this.currentBranch + 1 : 0;
		this.currentBranch = newBranch;
		this.#onBranchChange?.(newBranch);
	}
}

export function setMessageBranchContext(state: MessageBranchState) {
	setContext(MESSAGE_BRANCH_CONTEXT_KEY, state);
}

export function getMessageBranchContext(): MessageBranchState {
	const context = getContext<MessageBranchState | undefined>(MESSAGE_BRANCH_CONTEXT_KEY);
	if (!context) {
		throw new Error('MessageBranch components must be used within MessageBranch');
	}
	return context;
}
