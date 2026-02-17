import { InjectionToken } from '@eurora/shared/context';

export const DOWNLOAD_SERVICE = new InjectionToken<DownloadService>('DOWNLOAD_SERVICE');

interface PlatformInfo {
	url: string;
	signature: string;
}

export interface ReleaseInfoResponse {
	version: string;
	pub_date: string;
	platforms: Record<string, PlatformInfo>;
}

export type OSType = 'windows' | 'macos' | 'linux' | 'unknown';

const OS_TO_PLATFORM: Record<OSType, string[]> = {
	windows: ['windows-x86_64'],
	macos: ['darwin-aarch64', 'darwin-x86_64'],
	linux: ['linux-x86_64', 'linux-aarch64'],
	unknown: [],
};

export class DownloadService {
	private readonly baseUrl: string;
	private readonly channel: string;

	constructor(channel: string = 'release') {
		const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:50051';
		this.baseUrl = `${apiBaseUrl}`;
		this.channel = channel;
	}

	async getLatestRelease(): Promise<ReleaseInfoResponse | null> {
		try {
			const response = await fetch(`${this.baseUrl}/releases/${this.channel}`);

			if (response.status === 404) {
				return null;
			}

			if (!response.ok) {
				throw new Error(`Failed to fetch release info: ${response.statusText}`);
			}

			return await response.json();
		} catch (error) {
			console.error('Failed to get latest release:', error);
			throw error;
		}
	}

	async getDownloadUrl(os: OSType): Promise<string | null> {
		const release = await this.getLatestRelease();

		if (!release) {
			return null;
		}

		const platformKeys = OS_TO_PLATFORM[os];

		for (const platformKey of platformKeys) {
			const platform = release.platforms[platformKey];
			if (platform?.url) {
				return platform.url;
			}
		}

		return null;
	}

	async initiateDownload(os: OSType): Promise<boolean> {
		try {
			const url = await this.getDownloadUrl(os);

			if (!url) {
				console.warn(`No download available for OS: ${os}`);
				return false;
			}

			window.location.href = url;
			return true;
		} catch (error) {
			console.error('Failed to initiate download:', error);
			return false;
		}
	}

	async getAvailablePlatforms(): Promise<Record<string, boolean>> {
		const release = await this.getLatestRelease();

		if (!release) {
			return {};
		}

		const availability: Record<string, boolean> = {};
		for (const platform of Object.keys(release.platforms)) {
			availability[platform] = true;
		}

		return availability;
	}

	async getLatestVersion(): Promise<string | null> {
		const release = await this.getLatestRelease();
		return release?.version ?? null;
	}
}
