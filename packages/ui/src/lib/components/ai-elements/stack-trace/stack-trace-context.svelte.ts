import { getContext, setContext } from 'svelte';
import { parseStackTrace, type ParsedStackTrace } from './parse-stack.js';

const STACK_TRACE_CONTEXT_KEY = Symbol.for('ai-stack-trace');

export class StackTraceState {
	#raw = $state('');
	#trace = $state<ParsedStackTrace>({ errorType: null, errorMessage: '', frames: [], raw: '' });
	#isOpen = $state(false);
	#onFilePathClick: ((filePath: string, line?: number, column?: number) => void) | undefined;

	constructor(options: {
		raw: string;
		isOpen?: boolean;
		onFilePathClick?: (filePath: string, line?: number, column?: number) => void;
	}) {
		this.#raw = options.raw;
		this.#trace = parseStackTrace(options.raw);
		this.#isOpen = options.isOpen ?? false;
		this.#onFilePathClick = options.onFilePathClick;
	}

	get raw() {
		return this.#raw;
	}

	set raw(value: string) {
		this.#raw = value;
		this.#trace = parseStackTrace(value);
	}

	get trace() {
		return this.#trace;
	}

	get isOpen() {
		return this.#isOpen;
	}

	set isOpen(value: boolean) {
		this.#isOpen = value;
	}

	get onFilePathClick() {
		return this.#onFilePathClick;
	}

	set onFilePathClick(
		value: ((filePath: string, line?: number, column?: number) => void) | undefined,
	) {
		this.#onFilePathClick = value;
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
