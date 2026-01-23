// import { showError } from '$lib/notifications/toasts';
import { error as logErrorToFile } from '@tauri-apps/plugin-log';

// If you don't want to use Session Replay, remove the `Replay` integration,
// `replaysSessionSampleRate` and `replaysOnErrorSampleRate` options.

// Handler for unhandled errors inside promises.
window.onunhandledrejection = (e: PromiseRejectionEvent) => {
	logError(e.reason);
};

function logError(error: unknown) {
	try {
		// captureException(error, {
		// 	mechanism: {
		// 		type: 'sveltekit',
		// 		handled: false
		// 	}
		// });
		// showError('Unhandled exception', error);
		logErrorToFile(String(error));
	} catch (err: unknown) {
		console.error('Error while trying to log error.', err);
	}
}
