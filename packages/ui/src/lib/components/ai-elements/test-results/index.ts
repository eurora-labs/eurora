import Root from './test-results.svelte';
import Header from './test-results-header.svelte';
import Title from './test-results-title.svelte';
import Summary from './test-results-summary.svelte';
import Duration from './test-results-duration.svelte';
import Progress from './test-results-progress.svelte';
import Content from './test-results-content.svelte';
import Suite from './test-suite.svelte';
import SuiteHeader from './test-suite-header.svelte';
import SuiteTitle from './test-suite-title.svelte';
import SuiteStats from './test-suite-stats.svelte';
import SuiteContent from './test-suite-content.svelte';
import Case from './test-case.svelte';
import CaseHeader from './test-case-header.svelte';
import CaseTitle from './test-case-title.svelte';
import CaseStatus from './test-case-status.svelte';
import CaseContent from './test-case-content.svelte';
import CaseError from './test-case-error.svelte';

export {
	Root,
	Header,
	Title,
	Summary,
	Duration,
	Progress,
	Content,
	Suite,
	SuiteHeader,
	SuiteTitle,
	SuiteStats,
	SuiteContent,
	Case,
	CaseHeader,
	CaseTitle,
	CaseStatus,
	CaseContent,
	CaseError,
	//
	Root as TestResults,
	Header as TestResultsHeader,
	Title as TestResultsTitle,
	Summary as TestResultsSummary,
	Duration as TestResultsDuration,
	Progress as TestResultsProgress,
	Content as TestResultsContent,
	Suite as TestSuite,
	SuiteHeader as TestSuiteHeader,
	SuiteTitle as TestSuiteTitle,
	SuiteStats as TestSuiteStats,
	SuiteContent as TestSuiteContent,
	Case as TestCase,
	CaseHeader as TestCaseHeader,
	CaseTitle as TestCaseTitle,
	CaseStatus as TestCaseStatus,
	CaseContent as TestCaseContent,
	CaseError as TestCaseError,
};

export {
	type TestStatus,
	type TestResultsSummary as TestResultsSummaryType,
	TestResultsState,
	TestSuiteState,
	TestCaseState,
	getTestResultsContext,
	setTestResultsContext,
	getTestSuiteContext,
	setTestSuiteContext,
	getTestCaseContext,
	setTestCaseContext,
	formatDuration,
} from './test-results-context.svelte.js';
