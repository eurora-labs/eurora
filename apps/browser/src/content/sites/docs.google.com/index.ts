import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { textContext } from '../../../shared/content/tools/context';
import { googleDocsTools } from '../../../shared/content/tools/google_docs';
import {
	detectDocKind,
	getDocTitle,
	siteName,
} from '../../../shared/content/tools/google_docs/_lib';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// Google Docs content-script bundle. On an actual document or
/// spreadsheet the watcher exposes `[...webTools, ...googleDocsTools]`;
/// off-product pages (the file picker, account settings) just see the
/// generic web tools.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(
			() => (detectDocKind() !== null ? [...webTools, ...googleDocsTools] : webTools),
			() => {
				const kind = detectDocKind();
				if (kind === null) {
					return textContext('The user is on Google Docs (no document open).');
				}
				const title = getDocTitle().trim();
				const product = siteName(kind);
				return textContext(
					title
						? `The user is currently editing a ${product} document titled "${title}".`
						: `The user is currently editing a ${product} document.`,
				);
			},
		),
	);
}
