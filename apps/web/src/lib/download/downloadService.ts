export type OSType = 'windows' | 'macos' | 'linux' | 'unknown';
export type ArchType = 'x86_64' | 'aarch64' | 'unknown';

type TargetArch = string;

export interface DownloadOption {
	os: OSType;
	arch: ArchType;
	targetArch: TargetArch;
	label: string;
	archLabel: string;
}

const ALL_DOWNLOAD_OPTIONS: DownloadOption[] = [
	{
		os: 'windows',
		arch: 'x86_64',
		targetArch: 'windows-x86_64',
		label: 'Windows',
		archLabel: 'Intel/AMD (x64)',
	},
	{
		os: 'macos',
		arch: 'aarch64',
		targetArch: 'darwin-aarch64',
		label: 'macOS',
		archLabel: 'Apple Silicon',
	},
	{
		os: 'macos',
		arch: 'x86_64',
		targetArch: 'darwin-x86_64',
		label: 'macOS',
		archLabel: 'Intel',
	},
	{ os: 'linux', arch: 'x86_64', targetArch: 'linux-x86_64', label: 'Linux', archLabel: 'x64' },
	{
		os: 'linux',
		arch: 'aarch64',
		targetArch: 'linux-aarch64',
		label: 'Linux',
		archLabel: 'ARM (aarch64)',
	},
];

export function getDownloadOptions(os: OSType, arch: ArchType): DownloadOption[] {
	if (os === 'unknown') {
		// Neither platform nor arch detected - show all
		return ALL_DOWNLOAD_OPTIONS;
	}

	const osOptions = ALL_DOWNLOAD_OPTIONS.filter((o) => o.os === os);

	if (arch !== 'unknown') {
		// Both platform and arch detected - show exact match
		const exact = osOptions.filter((o) => o.arch === arch);
		if (exact.length > 0) return exact;
	}

	// Only platform detected - show all arch variants for that OS
	return osOptions;
}

/**
 * Builds a download URL that returns a 302 redirect to the presigned S3 artifact.
 * The browser will follow the redirect and start the download automatically.
 */
export function getDownloadUrl(option: DownloadOption, channel: string = 'release'): string {
	const baseUrl = import.meta.env.VITE_REST_API_URL ?? 'http://localhost:3000';
	return `${baseUrl}/download/${channel}/${option.targetArch}`;
}

/** Convenience: build a download URL from os+arch strings (kept for DownloadButton). */
export function getDownloadUrlForOS(
	os: Exclude<OSType, 'unknown'>,
	arch: ArchType = 'unknown',
	channel: string = 'release',
): string {
	const options = getDownloadOptions(os, arch);
	return getDownloadUrl(options[0], channel);
}
