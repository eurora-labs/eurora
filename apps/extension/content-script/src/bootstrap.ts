let loaded = false;
chrome.runtime.onMessage.addListener(async (msg) => {
	if (loaded || msg?.type !== 'SITE_LOAD') return;
	loaded = true;

	const imp = (p: string) => import(chrome.runtime.getURL(p));
	const runDefault = async () => {
		try {
			const def = await imp(msg.defaultChunk);
			def?.mainDefault?.();
		} catch {}
	};

	try {
		const mod = await imp(msg.chunk);
		const ok =
			(typeof mod.canHandle === 'function' ? !!mod.canHandle(document) : true) &&
			(typeof mod.main === 'function' ? (mod.main() ?? true) : true);
		if (!ok) await runDefault();
	} catch {
		await runDefault();
	}
});
