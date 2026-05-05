const TAG = '[eurora-office]';

export function info(...args: unknown[]): void {
	if (import.meta.env.DEV) {
		console.warn(TAG, ...args);
	}
}

export function warn(...args: unknown[]): void {
	console.warn(TAG, ...args);
}

export function error(...args: unknown[]): void {
	console.error(TAG, ...args);
}
