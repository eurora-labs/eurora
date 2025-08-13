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

export async function getCurrentTab() {
	try {
		const tabInfo = await browser.tabs.query({
			currentWindow: true,
			active: true,
		});

		return tabInfo[0];
	} catch (error) {
		console.error('Error getting current tab:', error);
		return null;
	}
}

function onMessageListener(message: any, sender: any) {
	console.log('Received from native app:', message);
	switch (message.type) {
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
			throw new Error(`Unknown message type: ${message.type}`);
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
	// Get the current active tab
	const activeTab = await getCurrentTab();

	if (!activeTab || !activeTab.id) {
		return { success: false, error: 'No active tab found', tab: activeTab };
	}

	type Response = {
		error?: string;
		[key: string]: any;
	};

	const response: Response = await new Promise((resolve, reject) =>
		browser.tabs.sendMessage(
			activeTab.id,
			{ type: messageType },
			// @ts-expect-error
			(response: any) => {
				if (browser.runtime.lastError) {
					reject({ error: browser.runtime.lastError });
				} else if (response?.error) {
					reject({ error: response.error });
				} else {
					resolve(response);
				}
			},
		),
	);

	return { success: true, ...response };
}

console.log('Extension background services finished');
