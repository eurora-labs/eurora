import { NativeMessenger } from '@eurora/browser-shared/background/native-messenger';

const messenger = new NativeMessenger();
messenger.start();

// console.log('Extension background services started');

// let nativePort: browser.runtime.Port | null = null;

// browser.tabs.onUpdated.addListener(async (tabId, changeInfo, tab) => {
// 	if (!nativePort) return;

// 	await onUpdated(tabId, changeInfo, tab, nativePort);
// });

// browser.tabs.onActivated.addListener(async (activeInfo) => {
// 	if (!nativePort) return;

// 	await onActivated(activeInfo.tabId, nativePort);
// });

// connect();

// function connect() {
// 	nativePort = browser.runtime.connectNative('com.eurora.app');
// 	console.log('Native port:', nativePort);
// 	const error = browser.runtime.lastError;
// 	if (error) {
// 		console.error('Native port connection failed:', error);
// 		return;
// 	}
// 	nativePort.onDisconnect.addListener(onDisconnectListener);
// 	nativePort.onMessage.addListener(onMessageListener as any);
// }

// function addBase64Prefix(base64: string) {
// 	const head = base64.substring(0, 6);
// 	switch (head) {
// 		case 'PHN2Zy':
// 			return `data:image/svg+xml;base64,${base64}`;
// 		case 'CiAgPH':
// 			return `data:image/svg+xml;base64,${base64.substring(4)}`;

// 		default:
// 			return base64;
// 	}
// }

// async function onMessageListener(frame: Frame, sender: browser.runtime.Port) {
// 	console.log('Received frame:', frame);

// 	let frameId = 0;
// 	// For now this is fine as Firefox doesn't send messages expecting a response
// 	if ('Request' in frame.id) {
// 		frameId = frame.id.Request;
// 	} else {
// 		throw new Error('Invalid frame ID: ' + frame.id);
// 	}

// 	switch (frame.command) {
// 		case 'GET_METADATA':
// 			try {
// 				const [activeTab] = await browser.tabs.query({ active: true, currentWindow: true });
// 				const iconBase64 = addBase64Prefix(await getCurrentTabIcon(activeTab));

// 				console.log('Tab metadata:', { url: activeTab.url, icon_base64: iconBase64 });

// 				const responseData = {
// 					kind: 'NativeMetadata',
// 					data: {
// 						url: activeTab.url,
// 						icon_base64: iconBase64,
// 					},
// 				};

// 				const responseFrame: Frame = {
// 					id: { Response: frameId },
// 					command: frame.command,
// 					payload: JSON.stringify(responseData),
// 				};

// 				sender.postMessage(responseFrame);
// 			} catch (error) {
// 				console.error('Error getting tab metadata:', error);

// 				const errorFrame: Frame = {
// 					id: { Error: frameId },
// 					command: frame.command,
// 					payload: undefined,
// 				};

// 				sender.postMessage(errorFrame);
// 			}
// 			break;
// 		default:
// 			try {
// 				// Handle assets request using the existing handleMessage
// 				const response = await handleMessage(frame.command);
// 				console.log('Finished responding to ', frame.command, ': ', response);

// 				const responseFrame: Frame = {
// 					id: { Response: frameId },
// 					command: frame.command,
// 					payload: JSON.stringify(response),
// 				};

// 				sender.postMessage(responseFrame);
// 			} catch (error) {
// 				console.error('Error responding to ', frame.command, ': ', error);

// 				const errorFrame: Frame = {
// 					id: { Error: frameId },
// 					command: frame.command,
// 					payload: undefined,
// 				};

// 				sender.postMessage(errorFrame);
// 			}
// 			break;
// 	}
// 	return true;
// }

// function onDisconnectListener() {
// 	console.log('Native port disconnected');
// 	nativePort = null;
// 	const error = browser.runtime.lastError;
// 	if (error) {
// 		console.error('Native port disconnected:', error);
// 		return;
// 	}

// 	setTimeout(() => {
// 		connect();
// 	}, 5000);
// }

// console.log('Extension background services finished');
