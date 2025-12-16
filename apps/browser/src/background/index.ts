import browser from 'webextension-polyfill';
import { startNativeMessenger } from '@eurora/browser-shared/background/native-messenger';
import { webNavigationListener } from '@eurora/browser-shared/background/bg';

browser.webNavigation.onCommitted.addListener(({ tabId, url, frameId }) => {
	webNavigationListener(tabId, url, frameId).then((result) => console.log(result));
	return true;
});

startNativeMessenger();
