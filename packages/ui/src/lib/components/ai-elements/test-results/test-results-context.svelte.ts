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

export class TestResultsState {
	#summary = $state<TestResultsSummary | undefined>(undefined);

	constructor(options: { summary?: TestResultsSummary }) {
		this.#summary = options.summary;
	}

	get summary() {
		return this.#summary;
	}

	set summary(value: TestResultsSummary | undefined) {
		this.#summary = value;
	}
}

export class TestSuiteState {
	#name = $state('');
	#status = $state<TestStatus>('passed');

	constructor(options: { name: string; status: TestStatus }) {
		this.#name = options.name;
		this.#status = options.status;
	}

	get name() {
		return this.#name;
	}

	set name(value: string) {
		this.#name = value;
	}

	get status() {
		return this.#status;
	}

	set status(value: TestStatus) {
		this.#status = value;
	}
}

export class TestCaseState {
	#name = $state('');
	#status = $state<TestStatus>('passed');
	#duration = $state<number | undefined>(undefined);

	constructor(options: { name: string; status: TestStatus; duration?: number }) {
		this.#name = options.name;
		this.#status = options.status;
		this.#duration = options.duration;
	}

	get name() {
		return this.#name;
	}

	set name(value: string) {
		this.#name = value;
	}

	get status() {
		return this.#status;
	}

	set status(value: TestStatus) {
		this.#status = value;
	}

	get duration() {
		return this.#duration;
	}

	set duration(value: number | undefined) {
		this.#duration = value;
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
