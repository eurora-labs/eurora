import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { textContext } from '../../../shared/content/tools/context';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { readSelectionForContext } from '../../../shared/content/tools/selection';
import { webTools } from '../../../shared/content/tools/web';

/// Snapshot of the page-level state the default context summary cares
/// about. Captured at `GET_CONTEXT` time so the formatter — which has no
/// other DOM access — can be unit-tested with literal values.
export interface DefaultPageState {
	title: string;
	url: string;
	selection: string;
}

/// Read the page state needed to describe the default web context.
/// Trims the title and normalizes the selection (whitespace collapse,
/// truncation, whitespace-only → empty) so the formatter can do plain
/// string assembly.
function readPageState(): DefaultPageState {
	return {
		title: document.title.trim(),
		url: window.location.href,
		selection: readSelectionForContext(),
	};
}

/// Pure formatter for the default site summary. Always emits a title-
/// and-URL sentence; appends a highlight sentence only when there is a
/// non-empty selection. Exported so tests can exercise the full matrix
/// without faking `Selection` or `document.title`.
export function formatDefaultContext(state: DefaultPageState): string {
	const { title, url, selection } = state;
	const intro = title
		? `The user is on the web page "${title}" at ${url}.`
		: `The user is on the web page at ${url}.`;
	if (!selection) return intro;
	return `${intro} They have the following text highlighted: "${selection}".`;
}

let initialized = false;

/// Default content-script bundle injected on every `http(s)` page that
/// no site-specific bundle claims. Surfaces the generic web tool set and
/// a minimal page summary; the summary mentions the user's current text
/// selection when there is one, so the model can ground its reply on
/// what the user is pointing at.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(
			() => webTools,
			() => textContext(formatDefaultContext(readPageState())),
		),
	);
}

export { main as mainDefault };
