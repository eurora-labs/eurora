import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { resolveTwitterTools } from '../../../shared/content/tools/twitter';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// X (Twitter) content-script bundle. Surfaces the generic web tools
/// alongside the X-specific tools appropriate for the current page —
/// `resolveTwitterTools` is re-evaluated per `LIST_TOOLS` call so SPA
/// navigation between e.g. `/home` and `/<handle>/status/<id>` flips
/// the surface without a content-script reload.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(watcherFromTools(() => [...webTools, ...resolveTwitterTools()]));
}
