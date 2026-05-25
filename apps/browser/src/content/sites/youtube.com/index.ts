import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { webTools } from '../../../shared/content/tools/web';
import { youtubeWatchTools } from '../../../shared/content/tools/youtube';

let initialized = false;

/// Whether the active YouTube page is the watch page. The watch-page
/// tools (`youtube_get_current_timestamp`, `_get_transcript`,
/// `_get_current_frame`) only make sense there; on `/feed` or `/results`
/// the model just sees the generic web tools.
function isWatchPage(): boolean {
	return window.location.pathname === '/watch';
}

export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(() => (isWatchPage() ? [...webTools, ...youtubeWatchTools] : webTools)),
	);
}
