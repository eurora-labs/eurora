import { getCurrentTab } from '@eurora/browser-shared/tabs';
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
			handleTabMessage('GENERATE_ASSETS')
				.then((response) => {
					console.log('Sending GENERATE_REPORT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating report', error);
					sender.postMessage({ success: false, error: String(error) });
				});
			return true; // Indicates we'll call sendResponse asynchronously
		case 'GENERATE_SNAPSHOT':
			handleTabMessage('GENERATE_SNAPSHOT')
				.then((response) => {
					console.log('Sending GENERATE_SNAPSHOT_RESPONSE message', response);
					sender.postMessage(response);
				})
				.catch((error) => {
					console.log('Error generating snapshot', error);
					sender.postMessage({ success: false, error: String(error) });
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

async function handleTabMessage(messageType: string) {
	try {
		// Get the current active tab
		const activeTab = await getCurrentTab();
		console.log('Active tab:', activeTab);

		if (!activeTab || !activeTab.id) {
			return { kind: 'Error', data: 'No active tab found' };
		}

		const response = await browser.tabs.sendMessage(activeTab.id, { type: messageType });
		console.log('Async response:', response);

		return { success: true, ...response };
	} catch (error) {
		console.error('Error handling tab message:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}

console.log('Extension background services finished');
