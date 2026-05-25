import { installToolHandlers } from '../../../shared/content/tools/install';
import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// Google Docs content-script bundle. Currently surfaces only the
/// generic web tools — Docs-specific tools (insert paragraph, get
/// selection, comment, …) land here as they're ported off the legacy
/// protocol.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(watcherFromTools(() => webTools));
}
