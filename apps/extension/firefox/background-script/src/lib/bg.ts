import browser from 'webextension-polyfill';
import { webNavigationListener } from '@eurora/browser-shared/bg';

browser.webNavigation.onCommitted.addListener(({ tabId, url, frameId }) => {
	webNavigationListener(tabId, url, frameId).then((result) => console.log(result));
	return true;
});
