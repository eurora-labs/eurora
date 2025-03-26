import './lib/youtube-watcher.ts';

// Send message to service worker
chrome.runtime.sendMessage({
	type: 'SEND_TO_NATIVE',
	payload: {
		videoId: window.location.search?.split('v=')[1]?.split('&')[0] || '',
		transcript: ''
	}
});

// Listen for responses from service worker
chrome.runtime.onMessage.addListener((message) => {
	if (message.type === 'NATIVE_RESPONSE') {
		console.log('Received response from native app:', message.payload);
		// Handle the response from native app
		if (message.payload.status === 'error') {
			console.error('Native app error:', message.payload.error);
		} else if (message.payload.type === 'TRANSCRIPT_RECEIVED') {
			console.log('Transcript successfully processed by native app');
		}
	}
});
