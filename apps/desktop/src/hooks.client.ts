import { error as logErrorToFile } from '@tauri-apps/plugin-log';

window.onunhandledrejection = (e: PromiseRejectionEvent) => {
	logError(e.reason);
};

function logError(error: unknown) {
	try {
		logErrorToFile(String(error));
	} catch (err: unknown) {
		console.error('Error while trying to log error.', err);
	}
}
