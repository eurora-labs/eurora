import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// Default content-script bundle injected on every `http(s)` page that
/// no site-specific bundle claims. Surfaces the generic web tool set;
/// nothing more, nothing less.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(watcherFromTools(() => webTools));
}

export { main as mainDefault };
