import { getContext, setContext } from 'svelte';

export type TestStatus = 'passed' | 'failed' | 'skipped' | 'running';

export interface TestResultsSummary {
	passed: number;
	failed: number;
	skipped: number;
	total: number;
	duration?: number;
}

const TEST_RESULTS_CONTEXT_KEY = Symbol.for('ai-test-results');
const TEST_SUITE_CONTEXT_KEY = Symbol.for('ai-test-suite');
const TEST_CASE_CONTEXT_KEY = Symbol.for('ai-test-case');

export interface TestResultsStateOptions {
	summary?: () => TestResultsSummary | undefined;
}

export class TestResultsState {
	readonly #opts: TestResultsStateOptions;

	constructor(opts: TestResultsStateOptions) {
		this.#opts = opts;
	}

	get summary(): TestResultsSummary | undefined {
		return this.#opts.summary?.();
	}
}

export interface TestSuiteStateOptions {
	name: () => string;
	status: () => TestStatus;
}

export class TestSuiteState {
	readonly #opts: TestSuiteStateOptions;

	constructor(opts: TestSuiteStateOptions) {
		this.#opts = opts;
	}

	get name(): string {
		return this.#opts.name();
	}

	get status(): TestStatus {
		return this.#opts.status();
	}
}

export interface TestCaseStateOptions {
	name: () => string;
	status: () => TestStatus;
	duration?: () => number | undefined;
}

export class TestCaseState {
	readonly #opts: TestCaseStateOptions;

	constructor(opts: TestCaseStateOptions) {
		this.#opts = opts;
	}

	get name(): string {
		return this.#opts.name();
	}

	get status(): TestStatus {
		return this.#opts.status();
	}

	get duration(): number | undefined {
		return this.#opts.duration?.();
	}
}

export function setTestResultsContext(state: TestResultsState) {
	setContext(TEST_RESULTS_CONTEXT_KEY, state);
}

export function getTestResultsContext(): TestResultsState {
	const context = getContext<TestResultsState | undefined>(TEST_RESULTS_CONTEXT_KEY);
	if (!context) {
		throw new Error('TestResults components must be used within TestResults');
	}
	return context;
}

export function setTestSuiteContext(state: TestSuiteState) {
	setContext(TEST_SUITE_CONTEXT_KEY, state);
}

export function getTestSuiteContext(): TestSuiteState {
	const context = getContext<TestSuiteState | undefined>(TEST_SUITE_CONTEXT_KEY);
	if (!context) {
		throw new Error('TestSuite components must be used within TestSuite');
	}
	return context;
}

export function setTestCaseContext(state: TestCaseState) {
	setContext(TEST_CASE_CONTEXT_KEY, state);
}

export function getTestCaseContext(): TestCaseState {
	const context = getContext<TestCaseState | undefined>(TEST_CASE_CONTEXT_KEY);
	if (!context) {
		throw new Error('TestCase components must be used within TestCase');
	}
	return context;
}

export function formatDuration(ms: number): string {
	if (ms < 1000) {
		return `${ms}ms`;
	}
	return `${(ms / 1000).toFixed(2)}s`;
}
