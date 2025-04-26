/**
 * Supported operating systems
 */
export enum OperatingSystem {
	WINDOWS = 'windows',
	MACOS = 'macos',
	LINUX = 'linux',
	UNKNOWN = 'unknown'
}

/**
 * Detect the current operating system
 */
export function detectOperatingSystem(): OperatingSystem {
	// Using Tauri's navigator.osType as primary method
	if (typeof window !== 'undefined' && window.__TAURI_METADATA__) {
		const osType = window.__TAURI_METADATA__.osType;

		if (osType.includes('windows')) return OperatingSystem.WINDOWS;
		if (osType.includes('macos') || osType.includes('darwin')) return OperatingSystem.MACOS;
		if (osType.includes('linux')) return OperatingSystem.LINUX;
	}

	// Fallback to navigator.platform for browsers
	if (typeof navigator !== 'undefined') {
		const platform = navigator.platform.toLowerCase();

		if (platform.includes('win')) return OperatingSystem.WINDOWS;
		if (platform.includes('mac')) return OperatingSystem.MACOS;
		if (platform.includes('linux') || platform.includes('unix')) return OperatingSystem.LINUX;
	}

	return OperatingSystem.UNKNOWN;
}

/**
 * Check if running in a Tauri app
 */
export function isTauri(): boolean {
	return typeof window !== 'undefined' && window.__TAURI_METADATA__ !== undefined;
}

// Add Tauri metadata to Window interface
declare global {
	interface Window {
		__TAURI_METADATA__?: {
			osType: string;
			[key: string]: any;
		};
	}
}
