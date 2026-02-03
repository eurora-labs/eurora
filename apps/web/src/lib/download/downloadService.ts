import { InjectionToken } from '@eurora/shared/context';

export const DOWNLOAD_SERVICE = new InjectionToken<DownloadService>('DOWNLOAD_SERVICE');

/**
 * Platform information from the release API
 */
interface PlatformInfo {
	url: string;
	signature: string;
}

/**
 * Response from the /releases/{channel} endpoint
 */
export interface ReleaseInfoResponse {
	version: string;
	pub_date: string;
	platforms: Record<string, PlatformInfo>;
}

/**
 * Operating system types
 */
export type OSType = 'windows' | 'macos' | 'linux' | 'unknown';

/**
 * Maps OS names to platform identifiers used by the backend.
 * Multiple variants are listed in preference order (e.g., arm64 first for macOS since Apple Silicon is more common now)
 * Uses "darwin" for macOS to match Tauri/Rust convention
 */
const OS_TO_PLATFORM: Record<OSType, string[]> = {
	windows: ['windows-x86_64'],
	macos: ['darwin-aarch64', 'darwin-x86_64'],
	linux: ['linux-x86_64', 'linux-aarch64'],
	unknown: [],
};

/**
 * Service for downloading the Eurora desktop application.
 * Connects to the be-update-service to get signed S3 URLs.
 */
export class DownloadService {
	private readonly baseUrl: string;
	private readonly channel: string;

	constructor(channel: string = 'release') {
		const apiBaseUrl = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:50051';
		this.baseUrl = `${apiBaseUrl}`;
		this.channel = channel;
	}

	/**
	 * Fetches the latest release information for the configured channel
	 */
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

	/**
	 * Gets the download URL for a specific operating system
	 * @param os - The operating system to get the download for
	 * @returns The signed S3 download URL or null if not available
	 */
	async getDownloadUrl(os: OSType): Promise<string | null> {
		const release = await this.getLatestRelease();

		if (!release) {
			return null;
		}

		const platformKeys = OS_TO_PLATFORM[os];

		// Try each platform variant for the OS (e.g., darwin-aarch64, darwin-x86_64)
		for (const platformKey of platformKeys) {
			const platform = release.platforms[platformKey];
			if (platform?.url) {
				return platform.url;
			}
		}

		return null;
	}

	/**
	 * Initiates a download for the user's detected operating system
	 * @param os - The operating system to download for
	 * @returns True if download was initiated, false otherwise
	 */
	async initiateDownload(os: OSType): Promise<boolean> {
		try {
			const url = await this.getDownloadUrl(os);

			if (!url) {
				console.warn(`No download available for OS: ${os}`);
				return false;
			}

			// Trigger download by navigating to the signed URL
			window.location.href = url;
			return true;
		} catch (error) {
			console.error('Failed to initiate download:', error);
			return false;
		}
	}

	/**
	 * Gets information about available platforms in the latest release
	 * @returns Map of platform identifiers to their availability
	 */
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

	/**
	 * Gets the latest version string
	 */
	async getLatestVersion(): Promise<string | null> {
		const release = await this.getLatestRelease();
		return release?.version ?? null;
	}
}
