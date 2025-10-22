import { handleGenerateAssets, handleGenerateSnapshot } from '@eurora/browser-shared/messaging';

console.log('Extension background services started');

let port: browser.runtime.Port | null = null;
connect();

function connect() {
	port = browser.runtime.connectNative('com.eurora.app');
	console.log('Native port:', port);
	const error = browser.runtime.lastError;
	if (error) {
		console.error('Native port connection failed:', error);
		return;
	}
	port.onDisconnect.addListener(onDisconnectListener);
	port.onMessage.addListener(onMessageListener as any);
}

function onMessageListener(message: any, sender: any) {
	console.log('Received from native app:', message);
	switch (message.command) {
		case 'GENERATE_ASSETS':
			handleGenerateAssets()
				.then((response) => {
					console.log('Sending GENERATE_REPORT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating report', error);
					sender.postMessage({ success: false, error: String(error), kind: 'Error' });
				});
			return true; // Indicates we'll call sendResponse asynchronously
		case 'GENERATE_SNAPSHOT':
			handleGenerateSnapshot()
				.then((response) => {
					console.log('Sending GENERATE_SNAPSHOT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating snapshot', error);
					sender.postMessage({ success: false, error: String(error), kind: 'Error' });
				});
			return true; // Indicates we'll call sendResponse asynchronously
		default:
			throw new Error(`Unknown message type: ${message.command}`);
	}
}

function onDisconnectListener() {
	console.log('Native port disconnected');
	port = null;
	const error = browser.runtime.lastError;
	if (error) {
		console.error('Native port disconnected:', error);
		return;
	}

	setTimeout(() => {
		connect();
	}, 5000);
}

console.log('Extension background services finished');
