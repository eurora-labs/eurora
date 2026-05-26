import { watcherFromTools } from '../../../shared/content/tools/build_watcher';
import { textContext } from '../../../shared/content/tools/context';
import { installToolHandlers } from '../../../shared/content/tools/install';
import { webTools } from '../../../shared/content/tools/web';

let initialized = false;

/// Default content-script bundle injected on every `http(s)` page that
/// no site-specific bundle claims. Surfaces the generic web tool set;
/// nothing more, nothing less.
export function main() {
	if (initialized) return;
	initialized = true;
	installToolHandlers(
		watcherFromTools(
			() => webTools,
			() => {
				const title = document.title.trim();
				const url = window.location.href;
				return textContext(
					title
						? `The user is on the web page "${title}" at ${url}.`
						: `The user is on the web page at ${url}.`,
				);
			},
		),
	);
}

export { main as mainDefault };
