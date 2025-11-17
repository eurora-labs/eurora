import { handleMessage } from '@eurora/browser-shared/background/messaging';
import { getCurrentTabIcon } from '@eurora/browser-shared/background/tabs';
import { onUpdated, onActivated } from '@eurora/browser-shared/background/focus-tracker';
import { Frame } from '@eurora/browser-shared/content/bindings';

console.log('Extension background services started');

let nativePort: browser.runtime.Port | null = null;

browser.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
	if (!nativePort) return;

	await onUpdated(tabId, changeInfo, tab, nativePort);
});

browser.tabs.onActivated.addListener(async (activeInfo) => {
	if (!nativePort) return;

	await onActivated(activeInfo.tabId, nativePort);
});

connect();

function connect() {
	nativePort = browser.runtime.connectNative('com.eurora.app');
	console.log('Native port:', nativePort);
	const error = browser.runtime.lastError;
	if (error) {
		console.error('Native port connection failed:', error);
		return;
	}
	nativePort.onDisconnect.addListener(onDisconnectListener);
	nativePort.onMessage.addListener(onMessageListener as any);
}

function addBase64Prefix(base64: string) {
	const head = base64.substring(0, 6);
	switch (head) {
		case 'PHN2Zy':
			return `data:image/svg+xml;base64,${base64}`;
		case 'CiAgPH':
			return `data:image/svg+xml;base64,${base64.substring(4)}`;

		default:
			return base64;
	}
}

async function onMessageListener(frame: Frame, sender: browser.runtime.Port) {
	console.log('Received frame:', frame);

	switch (frame.action) {
		case 'GET_METADATA':
			try {
				const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
				const iconBase64 = addBase64Prefix(await getCurrentTabIcon(activeTab));

				console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

				const responseData = {
					kind: 'NativeMetadata',
					data: {
						url: activeTab.url,
						icon_base64: iconBase64,
					},
				};

				const responseFrame: Frame = {
					kind: 'response',
					id: frame.id, // Echo back the request ID
					action: frame.action,
					event: '',
					payload: {
						kind: 'NativeMetadata',
						content: JSON.stringify(responseData),
					},
					ok: true,
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error getting tab metadata:', error);
				const errorFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: undefined,
					ok: false,
				};
				sender.postMessage(errorFrame);
			}
			break;
		default:
			try {
				// Handle assets request using the existing handleMessage
				const response = await handleMessage(frame.action);
				console.log('Finished responding to ', frame.action, ': ', response);

				const responseFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: {
						kind: response.kind || 'unknown',
						content: JSON.stringify(response),
					},
					ok: true,
				};

				sender.postMessage(responseFrame);
			} catch (error) {
				console.error('Error responding to ', frame.action, ': ', error);
				const errorFrame: Frame = {
					kind: 'response',
					id: frame.id,
					action: frame.action,
					event: '',
					payload: undefined,
					ok: false,
				};
				sender.postMessage(errorFrame);
			}
			break;
	}
	return true;
}

function onDisconnectListener() {
	console.log('Native port disconnected');
	nativePort = null;
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
