let loaded = false;
// @ts-ignore
const browserAny = typeof browser !== 'undefined' ? browser : (chrome as typeof browser);

browserAny.runtime.onMessage.addListener(
	(msg: any, sender: any, sendResponse: (response?: any) => void) => {
		listener(msg, sender, sendResponse)
			.then((result) => console.log(result))
			.catch((error) => console.error('Error in listener:', error));
		return true;
	},
);

async function listener(msg: any, sender: any, sendResponse: (response?: any) => void) {
	if (loaded || msg?.type !== 'SITE_LOAD') return false;
	loaded = true;
	document.documentElement.setAttribute('eurora-ext-ready', '1');

	const imp = async (p: string) => await import(browserAny.runtime.getURL(p));
	const runDefault = async () => {
		try {
			const def = await imp(msg.defaultChunk);
			def?.mainDefault?.();
		} catch (error) {
			console.error('Error loading default script:', error);
		}
	};

	try {
		const mod = await imp(msg.chunk);
		// For now this is unused but could be useful for some future websites
		const canHandle = typeof mod.canHandle === 'function' ? mod.canHandle(document) : true;

		// Execute the main function if present
		const mainResult = typeof mod.main === 'function' ? mod.main() : true;

		const ok = canHandle && (mainResult ?? true);

		if (ok) {
			document.documentElement.setAttribute('eurora-ext-site', msg.siteId);
			document.documentElement.setAttribute('eurora-ext-mounted', '1');
		}
		if (!ok) await runDefault();
	} catch (error) {
		console.error('Error loading site script:', error);
		await runDefault();
	}

	// Notify that the script is loaded and ready
	sendResponse({ loaded: true });
}
