import { UAParser } from 'ua-parser-js';
import type { OSType } from '$lib/download/downloadService';

export function getOS(): OSType {
	if (typeof navigator === 'undefined' || typeof window === 'undefined') {
		return 'unknown';
	}

	const parser = new UAParser(navigator.userAgent);
	const os = parser.getOS();
	const osName = os.name?.toLowerCase() ?? '';

	if (osName.includes('windows')) return 'windows';
	if (osName.includes('mac')) return 'macos';
	if (
		osName.includes('linux') ||
		osName.includes('ubuntu') ||
		osName.includes('debian') ||
		osName.includes('fedora') ||
		osName.includes('centos')
	)
		return 'linux';

	return 'unknown';
}

export function getOSDisplayName(os: OSType): string {
	switch (os) {
		case 'windows':
			return 'Windows';
		case 'macos':
			return 'macOS';
		case 'linux':
			return 'Linux';
		default:
			return 'Your OS';
	}
}
