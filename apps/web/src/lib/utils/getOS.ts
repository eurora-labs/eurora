import { UAParser } from 'ua-parser-js';
import type { ArchType, OSType } from '$lib/download/downloadService';

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

export function getArch(): ArchType {
	if (typeof navigator === 'undefined' || typeof window === 'undefined') {
		return 'unknown';
	}

	const ua = navigator.userAgent.toLowerCase();

	if (ua.includes('aarch64') || ua.includes('arm64')) return 'aarch64';

	if (typeof navigator.platform !== 'undefined') {
		const platform = navigator.platform.toLowerCase();
		if (platform.includes('arm') || platform.includes('aarch64')) return 'aarch64';
	}

	if ('userAgentData' in navigator && navigator.userAgentData) {
		const uaData = navigator.userAgentData as { platform?: string };
		if (uaData.platform === 'macOS') return 'unknown';
	}

	if (
		ua.includes('x86_64') ||
		ua.includes('x86-64') ||
		ua.includes('amd64') ||
		ua.includes('x64')
	) {
		return 'x86_64';
	}

	if (ua.includes('win64') || ua.includes('wow64')) return 'x86_64';

	if (ua.includes('linux') && !ua.includes('arm') && !ua.includes('aarch')) return 'x86_64';

	if (ua.includes('windows')) return 'x86_64';

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

export function getArchDisplayName(arch: ArchType): string {
	switch (arch) {
		case 'x86_64':
			return 'Intel/AMD (x64)';
		case 'aarch64':
			return 'ARM (Apple Silicon)';
		default:
			return '';
	}
}
