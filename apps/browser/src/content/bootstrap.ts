let loaded = false;
// @ts-expect-error - browser is not available in all contexts
const browserAny = typeof browser !== 'undefined' ? browser : (chrome as typeof browser);

browserAny.runtime.onMessage.addListener(
	(msg: any, sender: any, sendResponse: (response?: any) => void) => {
		if (msg?.type === 'HEARTBEAT') {
			sendResponse(true);
			return false;
		}
		if (loaded || msg?.type !== 'SITE_LOAD') return false;
		listener(msg, sender, sendResponse).catch((error) =>
			console.error('Error in listener:', error),
		);
		return true;
	},
);

async function listener(msg: any, sender: any, sendResponse: (response?: any) => void) {
	if (loaded || msg?.type !== 'SITE_LOAD') return false;
	loaded = true;
	document.documentElement.setAttribute('eurora-ext-ready', '1');

	async function imp(p: string) {
		return await import(browserAny.runtime.getURL(p));
	}
	async function runDefault() {
		try {
			const def = await imp(msg.defaultChunk);
			def?.mainDefault?.();
		} catch (error) {
			console.error('Error loading default script:', error);
		}
	}

	async function runCommon() {
		try {
			if (msg.commonChunk) {
				const common = await imp(msg.commonChunk);
				common?.main?.();
			}
		} catch (error) {
			console.error('Error loading common script:', error);
		}
	}

	await runCommon();

	try {
		const mod = await imp(msg.chunk);
		const canHandle = typeof mod.canHandle === 'function' ? mod.canHandle(document) : true;
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

	sendResponse({ loaded: true });
}
