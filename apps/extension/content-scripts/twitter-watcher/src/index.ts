import './lib/twitter-watcher.ts';

// Send message to service worker
chrome.runtime.sendMessage(
	{
		type: 'SEND_TO_NATIVE',
		payload: {
			url: window.location.href,
			tweets: [],
		},
	},
	(response) => {
		if (chrome.runtime.lastError) {
			console.error('Failed to send message to service worker:', chrome.runtime.lastError);
		}
	},
);

// Listen for responses from service worker
chrome.runtime.onMessage.addListener((message) => {
	if (message.type === 'NATIVE_RESPONSE' && message.payload) {
		console.log('Received response from native app:', message.payload);
		// Handle the response from native app
		if (message.payload.status === 'error') {
			console.error('Native app error:', message.payload.error || 'Unknown');
		} else if (message.payload.type === 'TWITTER_STATE_RECEIVED') {
			console.log('Twitter state successfully processed by native app');
		}
	}
});
