let loaded = false;
// @ts-ignore
const browserAny: typeof browser = typeof browser !== 'undefined' ? browser : (chrome as any);

browserAny.runtime.onMessage.addListener((msg, sender, sendResponse) => {
	listener(msg, sender, sendResponse).then((result) => console.log(result));
	return true;
});

// chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
// 	listener(msg, sender, sendResponse).then((result) => console.log(result));
// 	return true;
// });

async function listener(msg, sender, sendResponse) {
	if (loaded || msg?.type !== 'SITE_LOAD') return false;
	loaded = true;

	const imp = (p: string) => import(chrome.runtime.getURL(p));
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
		const ok =
			(typeof mod.canHandle === 'function' ? !!mod.canHandle(document) : true) &&
			(typeof mod.main === 'function' ? (mod.main() ?? true) : true);
		if (!ok) await runDefault();
	} catch (error) {
		console.error('Error loading site script:', error);
		await runDefault();
	}

	// Notify that the script is loaded and ready
	sendResponse({ loaded: true });
}
