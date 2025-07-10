console.log('Extension background services started');

let port = browser.runtime.connectNative('com.eurora.app');

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
/**
 *
 * Handles the GENERATE_REPORT message by getting the current active tab,
 * checking if it's a YouTube video or article page, and requesting a report
 * from the appropriate watcher
 */
async function handleGenerateReport() {
	try {
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
				{ type: 'GENERATE_ASSETS' },
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
	} catch (error) {
		console.error('Error generating report:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}

async function handleGenerateSnapshot() {
	try {
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
				{ type: 'GENERATE_SNAPSHOT' },
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
	} catch (error) {
		console.error('Error generating snapshot:', error);
		return {
			success: false,
			error: String(error),
		};
	}
}

// @ts-expect-error
port.onMessage.addListener(async (message: any, sender: any) => {
	console.log('Received from native app:', message);
	switch (message.type) {
		case 'GENERATE_ASSETS':
			handleGenerateReport()
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
			handleGenerateSnapshot()
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
});

port.onDisconnect.addListener(() => {
	const error = browser.runtime.lastError;
	if (error) {
		console.error('Native port disconnected:', error);
	} else {
		console.log('Native port disconnected');
	}
});

console.log('Extension background services finished');
