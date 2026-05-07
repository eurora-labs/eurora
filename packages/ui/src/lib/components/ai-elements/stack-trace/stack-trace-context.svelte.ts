import { getContext, setContext } from 'svelte';
import { parseStackTrace, type ParsedStackTrace } from './parse-stack.js';

const STACK_TRACE_CONTEXT_KEY = Symbol.for('ai-stack-trace');

export type StackTraceClickHandler = (filePath: string, line?: number, column?: number) => void;

export interface StackTraceStateOptions {
	raw: () => string;
	isOpen: () => boolean;
	setOpen: (value: boolean) => void;
	onFilePathClick?: () => StackTraceClickHandler | undefined;
}

export class StackTraceState {
	readonly #opts: StackTraceStateOptions;

	constructor(opts: StackTraceStateOptions) {
		this.#opts = opts;
	}

	get raw(): string {
		return this.#opts.raw();
	}

	get trace(): ParsedStackTrace {
		return parseStackTrace(this.raw);
	}

	get onFilePathClick(): StackTraceClickHandler | undefined {
		return this.#opts.onFilePathClick?.();
	}

	get isOpen(): boolean {
		return this.#opts.isOpen();
	}

	set isOpen(value: boolean) {
		this.#opts.setOpen(value);
	}
}

export function setStackTraceContext(state: StackTraceState) {
	setContext(STACK_TRACE_CONTEXT_KEY, state);
}

export function getStackTraceContext(): StackTraceState {
	const context = getContext<StackTraceState | undefined>(STACK_TRACE_CONTEXT_KEY);
	if (!context) {
		throw new Error('StackTrace components must be used within StackTrace');
	}
	return context;
}
