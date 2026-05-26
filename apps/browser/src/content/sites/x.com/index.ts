import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { textContext } from '../../../shared/content/tools/context';
import { installToolHandlers } from '../../../shared/content/tools/install';
import {
	getPageKind,
	resolveProfileHandle,
	resolveSearchQuery,
	resolveTwitterTools,
} from '../../../shared/content/tools/twitter';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// Per-page summary for the X bundle. Mirrors the routing in
/// `resolveTwitterTools` so the wording stays in lockstep with the
/// tool surface the LLM also sees.
function describeX(): string {
	const kind = getPageKind();
	switch (kind) {
		case 'home':
			return 'The user is browsing their X (Twitter) home timeline.';
		case 'profile': {
			const handle = resolveProfileHandle();
			return handle
				? `The user is currently viewing the X profile of @${handle}.`
				: 'The user is currently viewing an X profile.';
		}
		case 'search': {
			const query = resolveSearchQuery();
			return query ? `The user is searching X for "${query}".` : 'The user is searching X.';
		}
		case 'notifications':
			return 'The user is browsing their X notifications.';
		case 'tweet':
			return 'The user is currently reading a tweet thread on X.';
		case 'unsupported':
			return 'The user is browsing X (formerly Twitter).';
	}
}

/// X (Twitter) content-script bundle. Surfaces the generic web tools
/// alongside the X-specific tools appropriate for the current page —
/// `resolveTwitterTools` is re-evaluated per `LIST_TOOLS` call so SPA
/// navigation between e.g. `/home` and `/<handle>/status/<id>` flips
/// the surface without a content-script reload.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(
			() => [...webTools, ...resolveTwitterTools()],
			() => textContext(describeX()),
		),
	);
}
