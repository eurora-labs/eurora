import { installToolHandlers } from '../../../shared/content/tools/install';
import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// X (Twitter) content-script bundle. Currently surfaces only the
/// generic web tools — X-specific tools (compose, list timeline posts,
/// reply, …) land here as they're ported off the legacy protocol.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(watcherFromTools(() => webTools));
}
