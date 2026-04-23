import { injectIntoAllTabs, webNavigationListener } from '../shared/background/bg';
import { startNativeMessenger } from '../shared/background/native-messenger';
import browser from 'webextension-polyfill';

browser.webNavigation.onCommitted.addListener(
	({ tabId, url, frameId }) => {
		webNavigationListener(tabId, url, frameId).catch(console.error);
	},
	{ url: [{ schemes: ['http', 'https', 'file'] }] },
);

browser.runtime.onInstalled.addListener(async ({ reason }) => {
	if (reason !== 'install' && reason !== 'update') return;
	await injectIntoAllTabs();
});

browser.runtime.onStartup.addListener(async () => {
	await injectIntoAllTabs();
});

browser.runtime.onMessage.addListener((message, _sender, sendResponse) => {
	if (message.type === 'FETCH_URL') {
		fetch(message.url)
			.then(async (res) => {
				if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
				return await res.text();
			})
			.then((text) => sendResponse({ ok: true, text }))
			.catch((err) => sendResponse({ ok: false, error: String(err) }));
		return true;
	}
	return false;
});

startNativeMessenger();
