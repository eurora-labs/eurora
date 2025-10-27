import { handleMessage } from '@eurora/browser-shared/messaging';

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
	handleMessage(message.command)
		.then((response) => {
			console.log('Finished responding to type: ', message.command);
			sender.postMessage(response);
		})
		.catch((error) => {
			console.error('Error responding to message', error);
			sender.postMessage({ success: false, error: String(error) });
		});
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
