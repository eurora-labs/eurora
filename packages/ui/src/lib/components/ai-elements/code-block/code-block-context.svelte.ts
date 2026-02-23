import { getContext, setContext } from 'svelte';

const CODE_BLOCK_CONTEXT_KEY = Symbol.for('code-block-context');

export class CodeBlockState {
	#code = $state('');

	constructor(code: string) {
		this.#code = code;
	}

	get code() {
		return this.#code;
	}

	set code(value: string) {
		this.#code = value;
	}
}

export function setCodeBlockContext(state: CodeBlockState) {
	setContext(CODE_BLOCK_CONTEXT_KEY, state);
}

export function getCodeBlockContext(): CodeBlockState {
	const context = getContext<CodeBlockState | undefined>(CODE_BLOCK_CONTEXT_KEY);
	if (!context) {
		throw new Error('CodeBlock components must be used within CodeBlock');
	}
	return context;
}
