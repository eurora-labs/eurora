export type OSType = 'windows' | 'macos' | 'linux' | 'unknown';
export type ArchType = 'x86_64' | 'aarch64' | 'unknown';
export type LinuxPackageFormat = 'deb' | 'rpm' | 'appimage';

type TargetArch = string;

export interface DownloadOption {
	os: OSType;
	arch: ArchType;
	targetArch: TargetArch;
	label: string;
	archLabel: string;
	bundleType?: string;
	formatLabel?: string;
}

const LINUX_FORMATS: { format: LinuxPackageFormat; label: string }[] = [
	{ format: 'deb', label: '.deb' },
	{ format: 'rpm', label: '.rpm' },
	{ format: 'appimage', label: 'AppImage' },
];

function makeLinuxOptions(arch: ArchType, targetArch: string, archLabel: string): DownloadOption[] {
	return LINUX_FORMATS.map(({ format, label }) => ({
		os: 'linux' as const,
		arch,
		targetArch,
		label: 'Linux',
		archLabel,
		bundleType: format,
		formatLabel: label,
	}));
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
	...makeLinuxOptions('x86_64', 'linux-x86_64', 'x64'),
	...makeLinuxOptions('aarch64', 'linux-aarch64', 'ARM (aarch64)'),
];

export function detectLinuxFormat(): LinuxPackageFormat {
	return 'deb';
}

export function getDownloadOptions(os: OSType, arch: ArchType): DownloadOption[] {
	if (os === 'unknown') {
		return ALL_DOWNLOAD_OPTIONS;
	}

	let options = ALL_DOWNLOAD_OPTIONS.filter((o) => o.os === os);

	if (arch !== 'unknown') {
		const exact = options.filter((o) => o.arch === arch);
		if (exact.length > 0) options = exact;
	}

	if (os === 'linux') {
		const detected = detectLinuxFormat();
		options = sortByPreferredFormat(options, detected);
	}

	return options;
}

function sortByPreferredFormat(
	options: DownloadOption[],
	preferred: LinuxPackageFormat,
): DownloadOption[] {
	return [...options].sort((a, b) => {
		if (a.bundleType === preferred && b.bundleType !== preferred) return -1;
		if (a.bundleType !== preferred && b.bundleType === preferred) return 1;
		return 0;
	});
}

export function getDownloadUrl(option: DownloadOption, channel: string = 'release'): string {
	const baseUrl = import.meta.env.VITE_REST_API_URL ?? 'http://localhost:3000';
	if (option.bundleType) {
		return `${baseUrl}/download/${channel}/${option.targetArch}/${option.bundleType}`;
	}
	return `${baseUrl}/download/${channel}/${option.targetArch}`;
}

export function getDownloadUrlForOS(
	os: Exclude<OSType, 'unknown'>,
	arch: ArchType = 'unknown',
	channel: string = 'release',
): string {
	const options = getDownloadOptions(os, arch);
	return getDownloadUrl(options[0], channel);
}
