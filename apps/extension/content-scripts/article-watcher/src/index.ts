import './lib/article-watcher.ts';

// Send message to service worker
chrome.runtime.sendMessage({
	type: 'SEND_TO_NATIVE',
	payload: {
		url: window.location.href,
		title: document.title,
		content: '',
	},
});

// Listen for responses from service worker
chrome.runtime.onMessage.addListener((message) => {
	if (message.type === 'NATIVE_RESPONSE') {
		console.log('Received response from native app:', message.payload);
		// Handle the response from native app
		if (message.payload.status === 'error') {
			console.error('Native app error:', message.payload.error);
		} else if (message.payload.type === 'ARTICLE_RECEIVED') {
			console.log('Article successfully processed by native app');
		}
	}
});
