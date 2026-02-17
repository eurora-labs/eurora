import { webNavigationListener } from '../shared/background/bg';
import { startNativeMessenger } from '../shared/background/native-messenger';
import browser from 'webextension-polyfill';

browser.webNavigation.onCommitted.addListener(({ tabId, url, frameId }) => {
	webNavigationListener(tabId, url, frameId).catch(console.error);
	return true;
});

startNativeMessenger();
