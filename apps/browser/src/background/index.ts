import { webNavigationListener } from '@eurora/browser-shared/background/bg';
import { startNativeMessenger } from '@eurora/browser-shared/background/native-messenger';
import browser from 'webextension-polyfill';

browser.webNavigation.onCommitted.addListener(({ tabId, url, frameId }) => {
	webNavigationListener(tabId, url, frameId).catch(console.error);
	return true;
});

startNativeMessenger();
