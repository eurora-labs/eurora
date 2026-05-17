import { getContext, setContext } from 'svelte';

const CODE_BLOCK_CONTEXT_KEY = Symbol.for('code-block-context');

export interface CodeBlockStateOptions {
	code: () => string;
}

export class CodeBlockState {
	readonly #opts: CodeBlockStateOptions;

	constructor(opts: CodeBlockStateOptions) {
		this.#opts = opts;
	}

	get code(): string {
		return this.#opts.code();
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
